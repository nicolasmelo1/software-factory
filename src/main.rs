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
mod interview;
mod lang;
mod policy;
mod ratchet;
mod report;
mod scan;
mod skills;
mod verify;

use anyhow::Result;
use catalog::Catalog;
use checks::Ctx;
use clap::{Parser, Subcommand, ValueEnum};
use finding::{EXIT_BOOTSTRAP, EXIT_CONFIG, EXIT_OK};
use policy::{Policy, RULES_DIR};
use ratchet::Ratchet;
use std::path::{Path, PathBuf};
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
        /// Answers from a `factory-init` interview. Without them `init`
        /// scaffolds the default layers and nothing is tailored to this
        /// repository.
        #[arg(long)]
        answers: Option<PathBuf>,
    },
    /// Run every enabled rule.
    Check {
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
        /// Git ref to diff against, so gates activate from touched paths and
        /// the policy can be compared with the one being replaced.
        #[arg(long)]
        changed: Option<String>,
        /// Run one rule only.
        #[arg(long)]
        rule: Option<String>,
        /// Let `command` rules actually run. Off by default: a policy travels
        /// with a clone, so `sf check` must be safe on a repository you have
        /// not read.
        #[arg(long, env = "SF_ALLOW_COMMANDS")]
        allow_commands: bool,
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
    /// Write the mutation fixtures for every enabled rule.
    Fixtures,
    /// Regenerate the rule sections of docs/rules.md from the catalog.
    Docs,
    /// Install the agent skills that drive this tool.
    Skills {
        /// Where to write them. Without this, and without --project or
        /// --user, you are asked.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// This repository only: `<root>/.claude/skills`.
        #[arg(long, conflicts_with_all = ["dir", "user"])]
        project: bool,
        /// Every project on this machine: `~/.claude/skills`.
        #[arg(long, conflicts_with_all = ["dir", "project"])]
        user: bool,
    },
    /// Print the decision tree an interview walks, and what each answer does.
    Interview {
        /// Machine-readable, for an agent conducting the interview.
        #[arg(long)]
        json: bool,
    },
    /// Recompute the digests in a gate's evidence manifest.
    Seal { gate: String },
    /// Prove every enabled rule fires on its mutation fixture.
    Verify {
        #[arg(long)]
        rule: Option<String>,
        #[arg(long, env = "SF_ALLOW_COMMANDS")]
        allow_commands: bool,
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

fn changed_paths(root: &Path, base: &str) -> Result<Vec<String>> {
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
    // Split by whether the command writes: it keeps each arm list short, and
    // it is the distinction someone reading this actually wants.
    match cli.command {
        Cmd::Check { format, changed, rule, allow_commands } => {
            cmd_check(root, format, changed, rule, allow_commands)
        }
        Cmd::Verify { rule, allow_commands } => cmd_verify(root, rule, allow_commands),
        Cmd::Explain { rule } => cmd_explain(root, rule),
        Cmd::Catalog { layer } => cmd_catalog(root, layer),
        Cmd::Interview { json } => cmd_interview(json),
        writing => dispatch_writing(writing, root),
    }
}

fn dispatch_writing(command: Cmd, root: PathBuf) -> Result<i32> {
    match command {
        Cmd::Init { name, language, layer, force, answers } => {
            cmd_init(root, name, language, layer, force, answers)
        }
        Cmd::Ratchet { months } => cmd_ratchet(root, months),
        Cmd::Lock => cmd_lock(root),
        Cmd::Fixtures => cmd_fixtures(root),
        Cmd::Docs => cmd_docs(root),
        Cmd::Seal { gate } => cmd_seal(root, gate),
        Cmd::Skills { dir, project, user } => cmd_skills(root, dir, project, user),
        // Every read-only command is handled above.
        _ => unreachable!("read-only command routed to the writing dispatcher"),
    }
}

fn local_catalog(root: &Path) -> Result<Catalog> {
    let mut catalog = Catalog::builtin()?;
    catalog.extend_from_dir(&root.join(RULES_DIR))?;
    Ok(catalog)
}

fn cmd_skills(root: PathBuf, dir: Option<PathBuf>, project: bool, user: bool) -> Result<i32> {
    let dir = match (dir, project, user) {
        (Some(dir), _, _) => dir,
        (None, true, _) => root.join(skills::project_dir()),
        (None, _, true) => skills::user_dir()?,
        (None, false, false) => skills::choose_dir(&root)?,
    };
    for path in skills::install(&dir)? {
        println!("wrote {path}");
    }
    println!(
        "\nIn your project, invoke the skill by name — it will not be reached for on its own:\n\n  \
         /factory-init set up software-factory in this repo"
    );
    Ok(EXIT_OK)
}

fn cmd_interview(json: bool) -> Result<i32> {
    let interview = interview::Interview::load()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&interview)?);
        return Ok(EXIT_OK);
    }
    for decision in &interview.decisions {
        println!("\n{} — {}", decision.id, decision.question);
        if !decision.depends_on.is_empty() {
            let gates: Vec<String> = decision
                .depends_on
                .iter()
                .map(|(k, v)| format!("{k} in [{}]", v.join(", ")))
                .collect();
            println!("  asked when: {}", gates.join(" and "));
        }
        if decision.free_text {
            println!("  free text, e.g. {}", decision.example.as_deref().unwrap_or(""));
        }
        for option in &decision.options {
            println!("  - {:<18} {}", option.id, option.label);
        }
    }
    println!(
        "\nAnswers go in a file like:\n\n\
         version: 1\nanswers:\n  kind: backend-service\n  architecture: layered\n\n\
         then: sf init --answers answers.yaml"
    );
    Ok(EXIT_OK)
}

fn cmd_init(
    root: PathBuf,
    name: Option<String>,
    language: Vec<String>,
    layer: Vec<String>,
    force: bool,
    answers_path: Option<PathBuf>,
) -> Result<i32> {
    let catalog = Catalog::builtin()?;
    let (plan, answers) = match &answers_path {
        Some(path) => {
            let answers = interview::Answers::load(path)?;
            let tree = interview::Interview::load()?;
            (Some(interview::plan(&tree, &answers)?), Some(answers))
        }
        None => (None, None),
    };
    let name = name.unwrap_or_else(|| {
        root.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string())
    });
    let written = init::run(
        &root,
        &catalog,
        &init::InitOptions { name, languages: language, layers: layer, force, plan, answers },
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
    allow_commands: bool,
) -> Result<i32> {
    let loaded = load(root.clone())?;
    let base = changed.clone();
    let changed = match &base {
        Some(reference) => Some(changed_paths(&root, reference)?),
        None => None,
    };
    let ctx = Ctx {
        root: &loaded.root,
        policy: &loaded.policy,
        catalog: &loaded.catalog,
        files: &loaded.files,
        ratchet: &loaded.ratchet,
        changed,
        base: base.clone(),
        today: clock::today(),
        allow_commands,
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
                .get(policy::base_rule_id(id))
                .ok_or_else(|| anyhow::anyhow!("no rule {id} in the catalog"))?;
            Ok((checks::run_one(&checks::as_instance(rule, id), ctx)?, 1))
        }
        None => Ok((checks::run_all(ctx)?, loaded.policy.instances().len())),
    }
}

fn cmd_explain(root: PathBuf, rule: String) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    let found = catalog
        .get(policy::base_rule_id(&rule))
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

fn cmd_fixtures(root: PathBuf) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    let written = init::refresh_fixtures(&root, &catalog)?;
    println!("wrote {} fixture file(s)", written.len());
    Ok(EXIT_OK)
}

fn cmd_docs(root: PathBuf) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    init::refresh_rules_document(&root, &catalog)?;
    println!("regenerated docs/rules.md (everything above the first `## L` heading was preserved)");
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
        base: None,
        today: clock::today(),
        allow_commands: false,
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

fn cmd_verify(root: PathBuf, rule: Option<String>, allow_commands: bool) -> Result<i32> {
    let loaded = load(root)?;
    let outcomes =
        verify::run(&loaded.root, &loaded.policy, &loaded.catalog, rule.as_deref(), allow_commands)?;
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
