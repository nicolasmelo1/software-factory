//! `sf` — a portable software factory.
//!
//! The method this implements: every rule that matters is written twice, once
//! as prose that says why and once as a check that fails, and every check has
//! a mutation that proves it fires.

mod catalog;
mod checks;
mod clock;
mod digest;
mod finding;
mod fixtures;
mod init;
mod lang;
mod policy;
mod ratchet;
mod report;
mod scan;
mod verify;

use anyhow::Result;
use catalog::Catalog;
use checks::Ctx;
use clap::{Parser, Subcommand, ValueEnum};
use finding::{EXIT_BOOTSTRAP, EXIT_CONFIG, EXIT_OK};
use policy::{Policy, RULES_DIR};
use ratchet::Ratchet;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "sf",
    about = "Enforce a software development method across languages",
    long_about = "sf turns a language-neutral rule catalog into checks that fail, \
                  fixtures that prove those checks fire, and documentation that \
                  explains why each rule exists.",
    version
)]
struct Cli {
    /// Repository to operate on. Defaults to the enclosing git repository.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
    Markdown,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scaffold policy, docs, CI, hooks and mutation fixtures into a repository.
    Init {
        /// Project name recorded in the policy.
        #[arg(long)]
        name: Option<String>,
        /// Languages to parse: python, typescript, go.
        #[arg(long, value_delimiter = ',', default_values_t = ["python".to_string(), "typescript".to_string(), "go".to_string()])]
        language: Vec<String>,
        /// Layers to enable. L1, L4 and L5 are the honest day-one set: L0
        /// cements a shape you may not know yet, and L2/L3 need a second
        /// surface and a customer-visible flow to be about anything.
        #[arg(long, value_delimiter = ',', default_values_t = ["L1".to_string(), "L4".to_string(), "L5".to_string()])]
        layer: Vec<String>,
        /// Overwrite an existing policy.
        #[arg(long)]
        force: bool,
    },
    /// Run every enabled rule.
    Check {
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
        /// Git ref to diff against, so gates activate from touched paths.
        #[arg(long)]
        changed: Option<String>,
        /// Run one rule only.
        #[arg(long)]
        rule: Option<String>,
    },
    /// Print a rule: what it requires, why it exists, how to fix a violation.
    Explain { rule: String },
    /// List the catalog.
    Catalog {
        #[arg(long)]
        layer: Option<String>,
    },
    /// Freeze today's violations so a repository can adopt rules it breaks.
    Ratchet {
        /// Months until the frozen entries must be reviewed.
        #[arg(long, default_value_t = 6)]
        months: i64,
    },
    /// Rewrite the hash locks from what is on disk.
    Lock,
    /// Recompute the digests in a gate's evidence manifest.
    Seal { gate: String },
    /// Prove every enabled rule fires on its mutation fixture.
    Verify {
        #[arg(long)]
        rule: Option<String>,
    },
}

struct Loaded {
    root: PathBuf,
    policy: Policy,
    catalog: Catalog,
    ratchet: Ratchet,
    files: Vec<scan::SourceFile>,
}

fn load(root: PathBuf) -> Result<Loaded> {
    let policy = Policy::load(&root)?;
    let mut catalog = Catalog::builtin()?;
    catalog.extend_from_dir(&root.join(RULES_DIR))?;
    let ratchet = Ratchet::load(&root)?;
    let files = scan::walk(&root, &policy)?;
    Ok(Loaded { root, policy, catalog, ratchet, files })
}

fn changed_paths(root: &PathBuf, base: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--name-only", base])
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "git diff against {base} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .filter(|l| !l.is_empty())
        .collect())
}

fn main() {
    let cli = Cli::parse();
    let root = match cli.root.clone() {
        Some(path) => path,
        None => match policy::repo_root(&std::env::current_dir().unwrap_or_default()) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("sf: {e}");
                std::process::exit(EXIT_BOOTSTRAP);
            }
        },
    };
    match dispatch(cli, root) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("sf: {e:#}");
            std::process::exit(EXIT_CONFIG);
        }
    }
}

fn dispatch(cli: Cli, root: PathBuf) -> Result<i32> {
    match cli.command {
        Cmd::Init { name, language, layer, force } => cmd_init(root, name, language, layer, force),
        Cmd::Check { format, changed, rule } => cmd_check(root, format, changed, rule),
        Cmd::Explain { rule } => cmd_explain(root, rule),
        Cmd::Catalog { layer } => cmd_catalog(root, layer),
        Cmd::Ratchet { months } => cmd_ratchet(root, months),
        Cmd::Lock => cmd_lock(root),
        Cmd::Seal { gate } => cmd_seal(root, gate),
        Cmd::Verify { rule } => cmd_verify(root, rule),
    }
}

fn local_catalog(root: &PathBuf) -> Result<Catalog> {
    let mut catalog = Catalog::builtin()?;
    catalog.extend_from_dir(&root.join(RULES_DIR))?;
    Ok(catalog)
}

fn cmd_init(
    root: PathBuf,
    name: Option<String>,
    language: Vec<String>,
    layer: Vec<String>,
    force: bool,
) -> Result<i32> {
    let catalog = Catalog::builtin()?;
    let name = name.unwrap_or_else(|| {
        root.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string())
    });
    let written = init::run(
        &root,
        &catalog,
        &init::InitOptions { name, languages: language, layers: layer, force },
    )?;
    println!("wrote {} files:", written.len());
    for path in &written {
        println!("  {path}");
    }
    for path in init::update_locks(&root, &catalog)? {
        println!("  {path}");
    }
    let (_, frozen) = init::seed_ratchet(&root, &catalog, 6)?;
    println!("  {} ({frozen} existing violations frozen)", policy::RATCHET_PATH);
    println!(
        "\nnext:\n  \
         git config core.hooksPath .githooks\n  \
         sf verify          # prove the checks fire\n  \
         sf check           # see what is live\n  \
         sf explain <RULE>  # the reasoning behind any rule"
    );
    Ok(EXIT_OK)
}

fn cmd_check(
    root: PathBuf,
    format: Format,
    changed: Option<String>,
    rule: Option<String>,
) -> Result<i32> {
    let loaded = load(root.clone())?;
    let changed = match changed {
        Some(base) => Some(changed_paths(&root, &base)?),
        None => None,
    };
    let ctx = Ctx {
        root: &loaded.root,
        policy: &loaded.policy,
        catalog: &loaded.catalog,
        files: &loaded.files,
        ratchet: &loaded.ratchet,
        changed,
        today: clock::today(),
    };
    let (raw, rules_run) = run_selection(&loaded, &ctx, rule.as_deref())?;
    let (findings, frozen) = loaded.ratchet.apply(raw);
    let report = report::Report { findings, frozen, rules_run };
    match format {
        Format::Text => print!("{}", report.text(&loaded.catalog)),
        Format::Json => println!("{}", report.json()?),
        Format::Markdown => print!("{}", report.markdown(&loaded.catalog)),
    }
    Ok(report.exit_code())
}

/// One rule, or every enabled one.
fn run_selection(
    loaded: &Loaded,
    ctx: &Ctx,
    rule: Option<&str>,
) -> Result<(Vec<finding::Finding>, usize)> {
    match rule {
        Some(id) => {
            let rule = loaded
                .catalog
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("no rule {id} in the catalog"))?;
            Ok((checks::run_one(rule, ctx)?, 1))
        }
        None => Ok((
            checks::run_all(ctx)?,
            loaded.catalog.rules.keys().filter(|id| loaded.policy.enabled(id).is_some()).count(),
        )),
    }
}

fn cmd_explain(root: PathBuf, rule: String) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    let found = catalog
        .get(&rule)
        .ok_or_else(|| anyhow::anyhow!("no rule {rule} in the catalog"))?;
    println!(
        "{} [{}] {}\n\n{}\n\nWhy\n  {}\n\nFix\n  {}\n\nSeverity: {}  Ratchet: {}",
        found.id,
        found.layer.as_str(),
        found.title,
        found.statement,
        found.why,
        found.fix,
        found.severity,
        match found.ratchet {
            catalog::RatchetPolicy::Allowlist => "existing violations may be frozen",
            catalog::RatchetPolicy::None => "no grandfathering",
        }
    );
    Ok(EXIT_OK)
}

fn cmd_catalog(root: PathBuf, layer: Option<String>) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    for rule in catalog.rules.values() {
        if layer.as_deref().is_some_and(|l| l != rule.layer.as_str()) {
            continue;
        }
        println!("{:<4} {:<42} {}", rule.layer.as_str(), rule.id, rule.title);
    }
    Ok(EXIT_OK)
}

fn cmd_ratchet(root: PathBuf, months: i64) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    let (ratchet, frozen) = init::seed_ratchet(&root, &catalog, months)?;
    println!(
        "froze {frozen} violations across {} rules in {}",
        ratchet.rules.len(),
        policy::RATCHET_PATH
    );
    Ok(EXIT_OK)
}

fn cmd_lock(root: PathBuf) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    let written = init::update_locks(&root, &catalog)?;
    if written.is_empty() {
        println!("no enabled lock rule declares a scope — nothing to lock");
    }
    for path in written {
        println!("wrote {path}");
    }
    Ok(EXIT_OK)
}

fn cmd_seal(root: PathBuf, gate: String) -> Result<i32> {
    let loaded = load(root)?;
    let definition = loaded
        .policy
        .gates
        .get(&gate)
        .ok_or_else(|| anyhow::anyhow!("no gate {gate} in the policy"))?;
    let ctx = Ctx {
        root: &loaded.root,
        policy: &loaded.policy,
        catalog: &loaded.catalog,
        files: &loaded.files,
        ratchet: &loaded.ratchet,
        changed: None,
        today: clock::today(),
    };
    let manifest = checks::evidence::seal(&loaded.root, &gate, definition, &ctx)?;
    println!(
        "sealed {} — implementation {} over {} run(s)",
        definition.evidence,
        &manifest.implementation_sha256[..12],
        manifest.runs.len()
    );
    Ok(EXIT_OK)
}

fn cmd_verify(root: PathBuf, rule: Option<String>) -> Result<i32> {
    let loaded = load(root)?;
    let outcomes = verify::run(&loaded.root, &loaded.policy, &loaded.catalog, rule.as_deref())?;
    let mut broken = 0;
    for outcome in &outcomes {
        if outcome.fired {
            println!("\u{2713} {} — {}", outcome.rule, outcome.detail);
        } else {
            broken += 1;
            println!("\u{2717} {} — {}", outcome.rule, outcome.detail);
        }
    }
    println!("\n{}/{} enabled rules proven to fire", outcomes.len() - broken, outcomes.len());
    Ok(if broken == 0 { EXIT_OK } else { finding::EXIT_FINDINGS })
}
