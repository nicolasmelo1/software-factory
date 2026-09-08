//! `sf` — a portable software factory.
//!
//! The method this implements: every rule that matters is written twice, once
//! as prose that says why and once as a check that fails, and every check has
//! a mutation that proves it fires.

mod catalog;
mod checks;
mod clock;
mod digest;
mod docs;
mod escapes;
mod finding;
mod fingerprint;
mod fixtures;
mod init;
mod interview;
mod lang;
mod manifest;
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
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "sf",
    about = "Enforce a software development method across languages",
    long_about = "sf turns a language-neutral rule catalog into checks that fail, \
                  fixtures that prove those checks fire, and documentation that \
                  explains why each rule exists.",
    version = fingerprint::version_line()
)]
pub struct Cli {
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
        /// Languages to parse: python, typescript, go, rust, ruby.
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
        /// Where to write the rule reference. Defaults to `docs/rules.md`.
        #[arg(long)]
        rules_document: Option<String>,
        /// Answers from a `factory-init` interview. Without them `init`
        /// scaffolds the default layers and nothing is tailored to this
        /// repository.
        #[arg(long)]
        answers: Option<PathBuf>,
    },
    /// Run every enabled rule.
    Check {
        /// How to print the report: text for a person, json for a machine,
        /// markdown for a pull request comment.
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
        /// List one layer only, by its identifier: L0 through L6.
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
    /// Regenerate documentation. Add --check to make this read-only.
    Docs {
        /// Write nothing. Exit non-zero if any page would change, naming it.
        #[arg(long)]
        check: bool,
    },
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
        /// Prove one rule only.
        #[arg(long)]
        rule: Option<String>,
        /// Let `command` rules actually run, so a command rule can be proven
        /// to fire rather than reported as unproven.
        #[arg(long, env = "SF_ALLOW_COMMANDS")]
        allow_commands: bool,
    },
}

/// What this build of `sf` actually accepts: subcommand name -> the long
/// flags it takes, with the global ones folded in.
///
/// Read out of the clap definition above rather than written down anywhere,
/// because a second list of commands is a list that goes stale — which is the
/// defect `L4.RULE_PROSE_NAMES_A_REAL_COMMAND` exists to catch in prose.
pub fn accepted_commands() -> BTreeMap<String, BTreeSet<String>> {
    use clap::CommandFactory;
    let cli = Cli::command();
    // clap adds these two on build, and nothing here builds the command.
    let mut global = long_flags(&cli);
    global.insert("--help".to_string());
    global.insert("--version".to_string());
    cli.get_subcommands()
        .map(|sub| {
            let mut flags = long_flags(sub);
            flags.extend(global.iter().cloned());
            (sub.get_name().to_string(), flags)
        })
        .collect()
}

fn long_flags(command: &clap::Command) -> BTreeSet<String> {
    command.get_arguments().filter_map(|arg| arg.get_long()).map(|l| format!("--{l}")).collect()
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
        Cmd::Init { name, language, layer, force, answers, rules_document } => {
            cmd_init(root, name, language, layer, force, answers, rules_document)
        }
        Cmd::Ratchet { months } => cmd_ratchet(root, months),
        Cmd::Lock => cmd_lock(root),
        Cmd::Fixtures => cmd_fixtures(root),
        Cmd::Docs { check } => cmd_docs(root, check),
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
    let installed = skills::install(&dir)?;
    for path in installed.written {
        println!("wrote {path}");
    }
    for path in installed.removed {
        println!("removed retired skill {path}");
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
    rules_document: Option<String>,
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
        &init::InitOptions {
            name,
            languages: language,
            layers: layer,
            force,
            plan,
            answers,
            rules_document,
        },
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
        "\nnote: {}", init::FIXTURES_HINT
    );
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
    // The escape log runs on the working tree only: the trail belongs to an
    // in-progress attempt, so a run against a historical ref records
    // nothing. The log itself is derived state — a stale or corrupt entry
    // degrades the report, never the findings.
    let loggable = rule.is_none() && !escapes_logged(&root);
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

    // ---- the escape log's write path: entirely derived, never from prose ----
    let tree_diff = working_tree_diff(&loaded.root)?;
    let mut trail = BTreeMap::new();
    if loggable {
        trail = escape_bookkeeping(&loaded.root, &loaded.catalog, &findings, &tree_diff)?;
    }

    let report = report::Report { findings, frozen, rules_run, trail };
    match format {
        Format::Text => print!("{}", report.text(&loaded.catalog)),
        Format::Json => println!("{}", report.json()?),
        Format::Markdown => print!("{}", report.markdown(&loaded.catalog)),
    }
    Ok(report.exit_code())
}

/// The log is suppressed when this run is itself the capture of a red→green
/// transition: `sf check --changed <base>` against a tree mid-bisect would
/// record the bisect's own probes as the model's attempts. The marker file
/// is written by the capture path below.
fn escapes_logged(root: &Path) -> bool {
    root.join(escapes::DIR_NAME).join(".capture-in-progress").exists()
}

/// The digest of the current working-tree diff, or an empty string when git
/// is unavailable or the tree is clean. A stable summary is what makes "the
/// same edit twice" read as one attempt rather than two.
fn working_tree_diff(root: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "HEAD"])
        .output();
    let Ok(output) = output else {
        return Ok(String::new());
    };
    if !output.status.success() {
        return Ok(String::new());
    }
    let body = String::from_utf8_lossy(&output.stdout);
    Ok(if body.is_empty() { String::new() } else { digest::hex(body.as_bytes()) })
}

/// The red side of the write path. Every still-red key gains one attempt
/// (the current tree diff), and the capture runs for keys this run turned
/// green — the bisect that turns a fix into a worked repair.
fn escape_bookkeeping(
    root: &Path,
    catalog: &Catalog,
    findings: &[finding::Finding],
    tree_diff: &str,
) -> Result<BTreeMap<String, report::Trail>> {
    let mut trail = BTreeMap::new();
    let mut red_by_rule: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for finding in findings {
        let rule_id = policy::base_rule_id(&finding.rule);
        red_by_rule.entry(rule_id).or_default().push(finding.key.clone());
        if tree_diff.is_empty() {
            continue;
        }
        if let Some(label) = escapes::record_attempt(
            root,
            rule_id,
            &finding.key,
            tree_diff,
            Some(&snapshot_map(&loaded_snapshot(root)?)),
        )? {
            trail.insert(
                finding.key.clone(),
                report::Trail {
                    attempts: vec![format!(
                        "{label}: the working-tree diff {} was already tried against this finding",
                        &tree_diff[..tree_diff.len().min(12)]
                    )],
                    escape: None,
                },
            );
        }
    }
    // A pending key absent from this run just turned green. Capture it against
    // the current working tree before clearing its in-progress trail. Capture
    // failure is advisory-state failure, never a reason to fail `sf check`.
    if let Err(error) = capture_transitions(root, catalog, &red_by_rule) {
        eprintln!("sf: escape capture skipped: {error:#}");
    }
    // Build the report's view: attempts from the log, escapes past the
    // threshold, never a weakening.
    for finding in findings {
        let rule_id = policy::base_rule_id(&finding.rule);
        let attempts = escapes::attempts(root, rule_id, &finding.key);
        if attempts == 0 {
            continue;
        }
        let escape = escapes::retrieve(root, rule_id, &finding.key)?;
        trail.insert(
            finding.key.clone(),
            report::Trail {
                attempts: (1..=attempts)
                    .map(|n| format!("attempt {n} against this finding left it red"))
                    .collect(),
                escape,
            },
        );
    }
    Ok(trail)
}

/// The red→green capture. A key recorded red whose probe over the snapshot
/// tree is red but whose green tree is green has transitioned; the bisect
/// between the two trees is the escape.
fn capture_transitions(
    root: &Path,
    catalog: &Catalog,
    red_by_rule: &BTreeMap<&str, Vec<String>>,
) -> Result<()> {
    for rule_id in escapes::pending_rules(root)? {
        let Some(rule) = catalog.get(&rule_id) else { continue };
        let current = red_by_rule.get(rule_id.as_str());
        for key in escapes::pending_keys(root, &rule_id)? {
            if current.is_some_and(|keys| keys.contains(&key)) {
                continue;
            }
            let Some(snapshot) = escapes::snapshot_of(root, &rule_id, &key)? else {
                continue;
            };
            let mut green = BTreeMap::new();
            for path in snapshot.keys() {
                let abs = root.join(path);
                if let Ok(content) = std::fs::read_to_string(&abs) {
                    green.insert(path.clone(), content);
                }
            }
            let key_owned = key.clone();
            let rule_for_probe = rule.clone();
            let root_for_probe = root.to_path_buf();
            let probe = move |tree: &Path| -> anyhow::Result<Vec<String>> {
                probe_rule(&root_for_probe, &rule_for_probe, tree, &key_owned)
            };
            let captured = escapes::capture_escape(root, &rule_id, &key, &snapshot, &green, &probe)?;
            if captured {
                escapes::record_green(root, &rule_id, std::slice::from_ref(&key))?;
            }
            // A green key has no in-progress trail even if its diff was too
            // large or unexplainable to turn into a worked escape.
            escapes::clear_trail(root, &rule_id, &key)?;
        }
    }
    Ok(())
}

/// Run one rule over an arbitrary tree, returning its finding keys. The
/// bisect's probe: same policy bytes, same catalog, different files. The
/// scratch tree carries a copy of the real `.software-factory/policy.yaml`,
/// because a tree without a policy is not a repository this tool can read.
fn probe_rule(
    root: &Path,
    rule: &catalog::Rule,
    tree: &Path,
    _key: &str,
) -> anyhow::Result<Vec<String>> {
    let factory_dir = tree.join(".software-factory");
    std::fs::create_dir_all(&factory_dir)?;
    for name in ["policy.yaml", "ratchet.yaml"] {
        let source = root.join(".software-factory").join(name);
        if source.is_file() {
            std::fs::copy(&source, factory_dir.join(name))?;
        }
    }
    let policy = policy::Policy::load(tree)?;
    let files = scan::walk(tree, &policy)?;
    let ctx = Ctx {
        root: tree,
        policy: &policy,
        catalog: &local_catalog(root)?,
        files: &files,
        ratchet: &ratchet::Ratchet::default(),
        changed: None,
        base: None,
        today: clock::today(),
        allow_commands: false,
    };
    Ok(checks::run_one(rule, &ctx)?
        .into_iter()
        .map(|f| f.key)
        .collect())
}

/// The content of every file the working-tree diff touches — the snapshot a
/// red run leaves behind for a later green run to bisect against.
fn changed_files_snapshot(root: &Path) -> Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--name-only", "HEAD"])
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "git diff --name-only failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .filter(|l| !l.is_empty())
        .collect())
}

/// The snapshot actually stored: only files this binary can read as text,
/// capped so an oversized diff records no snapshot rather than a partial one.
fn loaded_snapshot(root: &Path) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for path in changed_files_snapshot(root)? {
        let abs = root.join(&path);
        if let (true, Ok(content)) = (abs.is_file(), std::fs::read_to_string(&abs)) {
            out.push((path, content));
        }
    }
    Ok(out)
}

/// A snapshot as the escape log stores it.
fn snapshot_map(files: &[(String, String)]) -> BTreeMap<String, String> {
    files.iter().cloned().collect()
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

fn cmd_docs(root: PathBuf, check: bool) -> Result<i32> {
    let catalog = local_catalog(&root)?;
    if check {
        let changed = docs::check(&root, &catalog)?;
        if changed.is_empty() {
            println!("documentation is up to date");
            Ok(EXIT_OK)
        } else {
            for path in changed { println!("would change {}", path.display()); }
            println!("run `sf docs` to update documentation");
            Ok(finding::EXIT_FINDINGS)
        }
    } else {
        init::refresh_rules_document(&root, &catalog)?;
        let written = docs::apply(&root, &catalog)?;
        println!("regenerated docs/rules.md (everything above the first `## L` heading was preserved)");
        for path in written { println!("updated {}", path.display()); }
        Ok(EXIT_OK)
    }
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
