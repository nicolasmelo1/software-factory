//! `sf init` — write the enforcement into the target repository.
//!
//! This is the part that matters for adoption: the tool does not ask a team to
//! hand-write policy, fixtures and CI. It generates them, seeds the ratchet
//! from what the repository looks like today, and leaves a diff a reviewer can
//! read.

use crate::catalog::{Catalog, Layer, RatchetPolicy};
use crate::checks::{self, Ctx, options_for};
use crate::clock;
use crate::fixtures;
use crate::policy::{FIXTURES_DIR, POLICY_PATH, Policy, RULES_DIR};
use crate::ratchet::Ratchet;
use crate::scan;
use anyhow::{Result, bail};
use std::collections::BTreeSet;
use std::path::Path;

pub struct InitOptions {
    pub name: String,
    pub languages: Vec<String>,
    pub layers: Vec<String>,
    pub force: bool,
    /// What an interview decided, if one was run. Without it `init` scaffolds
    /// the default layers and nothing is tailored to this repository.
    pub plan: Option<crate::interview::Plan>,
    pub answers: Option<crate::interview::Answers>,
    /// Where to write the rule reference, for repositories that already have
    /// a documentation convention of their own.
    pub rules_document: Option<String>,
}

pub fn run(root: &Path, catalog: &Catalog, opts: &InitOptions) -> Result<Vec<String>> {
    let policy_path = root.join(POLICY_PATH);
    if policy_path.exists() && !opts.force {
        bail!("{} already exists — pass --force to overwrite", policy_path.display());
    }

    // The interview can pull a rule in from a layer that was not selected —
    // saying "we use repositories" enables the L0 rule even on a day-one
    // L1/L4/L5 install, because the person just said the boundary is real.
    let selected = select_rules(catalog, opts);
    if selected.is_empty() {
        bail!("no rules match the requested layers");
    }

    let mut written = Vec::new();
    write(root, POLICY_PATH, &policy_document(opts, &selected, root), &mut written)?;
    write(root, opts.rules_document.as_deref().unwrap_or("docs/rules.md"), &rules_document(opts, &selected), &mut written)?;
    if let (Some(plan), Some(answers)) = (&opts.plan, &opts.answers) {
        write_from_interview(root, plan, answers, &mut written)?;
    }
    write_cadence_files(root, &selected, &mut written)?;
    write(
        root,
        ".github/workflows/software-factory.yml",
        &workflow(&opts.languages, &selected),
        &mut written,
    )?;
    write_automation(root, &mut written)?;
    write_fixtures(root, &selected, &mut written)?;
    Ok(written)
}

/// Rules the interview asked for that are not in the catalog: templates with
/// this repository's own names filled in, each with the fixture that proves it
/// fires, plus the record of who decided what.
fn write_from_interview(
    root: &Path,
    plan: &crate::interview::Plan,
    answers: &crate::interview::Answers,
    written: &mut Vec<String>,
) -> Result<()> {
    for name in &plan.templates {
        let built = crate::interview::instantiate(name, &plan.vars)?;
        write(root, &format!("{RULES_DIR}/{name}.yaml"), &built.body, written)?;
        let base = format!("{FIXTURES_DIR}/{}", built.rule.id);
        write(
            root,
            &format!("{base}/{POLICY_PATH}"),
            &fixtures::minimal_policy(&built.rule.id),
            written,
        )?;
        for (path, body) in &built.fixture {
            write(root, &format!("{base}/{path}"), body, written)?;
        }
    }
    write(root, "docs/architecture-decisions.md", &decision_record(answers)?, written)?;
    Ok(())
}

/// What was decided, in the words of the interview, next to the rules it
/// produced. A rule whose reason is only in someone's memory is a rule that
/// gets deleted the first time it is inconvenient.
fn decision_record(answers: &crate::interview::Answers) -> Result<String> {
    let interview = crate::interview::Interview::load()?;
    let mut out = String::from(
        "# Architecture decisions\n\n\
         Answers from the `factory-init` interview, and the rules each one\n\
         produced. Change an answer here and re-run `sf init --answers` rather\n\
         than editing the generated policy by hand: the answer is the decision,\n\
         the policy is its consequence.\n",
    );
    for (id, answer) in &answers.answers {
        let Some(decision) = interview.get(id) else { continue };
        let chosen = decision
            .options
            .iter()
            .find(|o| o.id == *answer)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| answer.clone());
        out.push_str(&format!("\n## {}\n\n**{}**\n\n{}\n", decision.question, chosen, decision.why));
        if let Some(note) =
            decision.options.iter().find(|o| o.id == *answer).and_then(|o| o.note.clone())
        {
            out.push_str(&format!("\n{note}\n"));
        }
    }
    Ok(out)
}

/// The files L4 rules need in order to be about anything.
fn write_cadence_files(
    root: &Path,
    selected: &[&crate::catalog::Rule],
    written: &mut Vec<String>,
) -> Result<()> {
    if selected.iter().any(|r| r.id == "L4.ROOT_FILES_ARE_DECLARED") {
        write(root, ".allowed-root-files", &root_allowlist(root)?, written)?;
    }
    if selected.iter().any(|r| r.id == "L4.PLAN_DECLARES_EXIT_CONDITION")
        && !root.join("plans/next-steps.md").exists()
    {
        write(root, "plans/next-steps.md", NEXT_STEPS, written)?;
    }
    Ok(())
}

fn write_automation(root: &Path, written: &mut Vec<String>) -> Result<()> {
    write(root, ".githooks/pre-commit", PRE_COMMIT, written)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let hook = root.join(".githooks/pre-commit");
        if hook.exists() {
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

fn write_fixtures(
    root: &Path,
    selected: &[&crate::catalog::Rule],
    written: &mut Vec<String>,
) -> Result<()> {
    for rule in selected {
        let Some(fixture) = fixtures::for_rule(&rule.id) else {
            continue;
        };
        let base = format!("{FIXTURES_DIR}/{}", rule.id);
        write(root, &format!("{base}/{POLICY_PATH}"), &fixtures::fixture_policy(fixture), written)?;
        for (path, body) in fixture.files {
            write(root, &format!("{base}/{path}"), body, written)?;
        }
    }
    Ok(())
}

/// Rules to switch on: the requested layers, plus anything an interview
/// answer justified, minus anything an answer ruled out. Saying "we use
/// repositories" turns the L0 rule on even on a day-one L1/L4/L5 install,
/// because the person just told you the boundary is real.
fn select_rules<'a>(catalog: &'a Catalog, opts: &InitOptions) -> Vec<&'a crate::catalog::Rule> {
    let enabled: Vec<String> =
        opts.plan.as_ref().map(|p| p.enable.iter().cloned().collect()).unwrap_or_default();
    let disabled: Vec<String> =
        opts.plan.as_ref().map(|p| p.disable.iter().cloned().collect()).unwrap_or_default();
    catalog
        .rules
        .values()
        .filter(|r| {
            (opts.layers.iter().any(|l| l == r.layer.as_str()) || enabled.contains(&r.id))
                && !disabled.contains(&r.id)
        })
        .collect()
}

fn write(root: &Path, rel: &str, body: &str, written: &mut Vec<String>) -> Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body)?;
    written.push(rel.to_string());
    Ok(())
}

/// Write the mutation fixtures for every enabled rule, without touching the
/// policy. Adding a rule should not mean re-scaffolding the repository.
pub fn refresh_fixtures(root: &Path, catalog: &Catalog) -> Result<Vec<String>> {
    let policy = Policy::load(root)?;
    let selected: Vec<&crate::catalog::Rule> =
        catalog.rules.values().filter(|r| policy.any_instance_enabled(&r.id)).collect();
    let mut written = Vec::new();
    write_fixtures(root, &selected, &mut written)?;
    Ok(written)
}

/// Freeze today's violations so the rules can be adopted by a repository that
/// already breaks them. New violations still fail from the first run.
pub fn seed_ratchet(root: &Path, catalog: &Catalog, months: i64) -> Result<(Ratchet, usize)> {
    let policy = Policy::load(root)?;
    let files = scan::walk(root, &policy)?;
    let empty = Ratchet::default();
    let ctx = Ctx {
        root,
        policy: &policy,
        catalog,
        files: &files,
        ratchet: &empty,
        changed: None,
        base: None,
        today: clock::today(),
        allow_commands: false,
    };
    let review_by = clock::plus_months(&ctx.today, months);
    let mut ratchet = Ratchet::default();
    let mut total = 0;
    for (instance, base) in policy.instances() {
        let Some(rule) = catalog.get(&base) else { continue };
        if rule.ratchet == RatchetPolicy::None {
            continue;
        }
        let rule = &checks::as_instance(rule, &instance);
        let (id, _) = (&instance, ());
        let keys: BTreeSet<String> = checks::run_one(rule, &ctx)?
            .into_iter()
            .map(|f| f.key)
            .collect();
        total += keys.len();
        ratchet.seed(id, keys, &review_by);
    }
    ratchet.save(root)?;
    Ok((ratchet, total))
}

/// Write the locks for every enabled lock rule that declares a scope.
pub fn update_locks(root: &Path, catalog: &Catalog) -> Result<Vec<String>> {
    let policy = Policy::load(root)?;
    let files = scan::walk(root, &policy)?;
    let ratchet = Ratchet::default();
    let ctx = Ctx {
        root,
        policy: &policy,
        catalog,
        files: &files,
        ratchet: &ratchet,
        changed: None,
        base: None,
        today: clock::today(),
        allow_commands: false,
    };
    let mut written = Vec::new();
    for (instance, base) in policy.instances() {
        let Some(rule) = catalog.get(&base) else { continue };
        if !matches!(rule.check, crate::catalog::CheckKind::Lock) {
            continue;
        }
        let rule = &checks::as_instance(rule, &instance);
        let opts = options_for(rule, &policy)?;
        if opts.scope.is_empty() {
            continue;
        }
        let Some(lock_file) = opts.lock_file.clone() else {
            continue;
        };
        let lock = checks::lock::current(&opts, &ctx)?;
        lock.save(&root.join(&lock_file))?;
        written.push(lock_file);
    }
    Ok(written)
}

/// Dependency manifests that actually exist here. A lock over files that are
/// not present is the same as no lock at all.
fn dependency_manifests(root: &Path) -> Vec<String> {
    ["package.json", "pyproject.toml", "requirements.txt", "go.mod", "Cargo.toml", "Cargo.lock", "Gemfile"]
        .iter()
        .filter(|name| root.join(name).exists())
        .map(|name| name.to_string())
        .collect()
}

fn interview_options(opts: &InitOptions, rule_id: &str) -> Option<String> {
    let value = opts.plan.as_ref()?.options.get(rule_id)?;
    let rendered = serde_yaml::to_string(value).ok()?;
    let indented: String =
        rendered.lines().map(|line| format!("      {line}\n")).collect();
    Some(format!("    options:\n{indented}"))
}

fn policy_document(opts: &InitOptions, selected: &[&crate::catalog::Rule], root: &Path) -> String {
    let mut out = String::new();
    out.push_str(
        "# Which rules this repository enforces, and how its paths map onto the\n\
         # catalog's neutral vocabulary. `sf explain <RULE>` prints the reasoning\n\
         # behind any rule below.\n\
         version: 1\n\
         project:\n",
    );
    out.push_str(&format!("  name: {}\n", opts.name));
    out.push_str(&format!("  languages: [{}]\n", opts.languages.join(", ")));
    out.push_str("  exclude: []\n\ndocs:\n  scan:\n    - \"docs/**/*.md\"\n    - \"*.md\"\n");
    if let Some(path) = &opts.rules_document {
        out.push_str(&format!("  rules_document: \"{path}\"\n"));
    }
    out.push('\n');
    out.push_str(
        "# A gate activates from the paths a change touches, never from a label\n\
         # or a sentence in a pull request. See L3.GATE_HAS_FRESH_EVIDENCE.\n\
         gates: {}\n\nrules:\n",
    );
    for rule in selected {
        out.push_str(&format!("  # {} — {}\n", rule.layer.as_str(), rule.title));
        out.push_str(&format!("  {}:\n    enabled: true\n", rule.id));
        if let Some(block) = interview_options(opts, &rule.id) {
            out.push_str(&block);
            continue;
        }
        // The guardrail's own lock ships with the right scope; the others
        // cannot be guessed, so they are filled from what is actually here.
        if rule.id == "L2.DEPENDENCIES_CHANGE_DELIBERATELY" {
            let manifests = dependency_manifests(root);
            out.push_str("    options:\n      scope:\n");
            for manifest in &manifests {
                out.push_str(&format!("        - \"{manifest}\"\n"));
            }
            if manifests.is_empty() {
                out.push_str("        # No dependency manifest found. Add yours, or disable the rule.\n");
            }
        } else if rule.id == "L2.GENERATED_FILES_ARE_LOCKED" {
            out.push_str(
                "    options:\n      # Nothing is locked until you say what is generated. An enabled\n      \
                 # lock with an empty scope is inert, and L5.NO_INERT_RULE will say so.\n      scope: []\n",
            );
        }
    }
    out
}

/// Regenerate the rule sections, preserving whatever a human wrote above them.
/// Everything before the first `## L` heading is this repository's own
/// reasoning and is never touched.
pub fn refresh_rules_document(root: &Path, catalog: &Catalog) -> Result<()> {
    let policy = Policy::load(root)?;
    let selected: Vec<&crate::catalog::Rule> =
        catalog.rules.values().filter(|r| policy.any_instance_enabled(&r.id)).collect();
    let path = root.join(policy.docs.rules_document());
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let preamble: String = existing
        .lines()
        .take_while(|line| !line.starts_with("## L"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = if preamble.trim().is_empty() {
        rules_preamble(&policy.project.name)
    } else {
        preamble.trim_end().to_string()
    };
    out.push('\n');
    out.push_str(&rule_sections(&selected));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, out)?;
    Ok(())
}

fn rules_preamble(name: &str) -> String {
    format!(
        "# Rules {} enforces\n\n\
         Generated by `sf init`. Every rule below is enforced in CI; this document\n\
         is the other half of the pair — the half that says why. Replace the\n\
         generic reasoning with this repository's own decision as it acquires one,\n\
         but do not delete a rule's section while the rule is enabled:\n\
         `L4.EVERY_RULE_HAS_A_WHY` fails when enforcement and prose come apart.\n",
        name
    )
}

fn rules_document(opts: &InitOptions, selected: &[&crate::catalog::Rule]) -> String {
    format!("{}\n{}", rules_preamble(&opts.name), rule_sections(selected))
}

fn rule_sections(selected: &[&crate::catalog::Rule]) -> String {
    let mut out = String::new();
    let mut layer = None;
    for rule in selected {
        if layer != Some(rule.layer) {
            layer = Some(rule.layer);
            out.push_str(&format!("\n## {} — {}\n", rule.layer.as_str(), layer_title(rule.layer)));
        }
        out.push_str(&format!(
            "\n### {}\n\n**{}**\n\n{}\n\n**Why.** {}\n\n**Fix.** {}\n",
            rule.id, rule.title, rule.statement, rule.why, rule.fix
        ));
    }
    out
}

fn layer_title(layer: Layer) -> &'static str {
    match layer {
        Layer::L0 => "Shape: where things live",
        Layer::L1 => "Grain: how the code reads",
        Layer::L2 => "Contract: no drift from the source of truth",
        Layer::L3 => "Effect: a real actor achieved the outcome",
        Layer::L4 => "Cadence: docs, plans and rules stay attached",
        Layer::L5 => "Meta: the guardrail is proven to fire",
        Layer::L6 => "Hazard: the defect classes this repository hunts",
    }
}

fn root_allowlist(root: &Path) -> Result<String> {
    let mut names: Vec<String> = std::fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    Ok(format!(
        "# Files allowed at the repository root. Adding a line here is the\n\
         # deliberate act; NOTES.md appearing without one is the reflex.\n{}\n",
        names.join("\n")
    ))
}

const NEXT_STEPS: &str = "# Next steps\n\n\
The execution order. One table, short on purpose: this is the file to reread\n\
weekly, and the file an agent reads to know what is next.\n\n\
A plan not listed here is written, valid, and off the critical path until its\n\
precondition exists. Park it in the second table rather than deleting it.\n\n\
| # | Work | Exit condition |\n| --- | --- | --- |\n| 1 | _first plan_ | _the externally visible effect that ends it_ |\n\n\
## Parked\n\n| Work | Waiting on |\n| --- | --- |\n";

/// The workflow, with the hazard tooling wired in for the languages this
/// repository actually uses. Tools that need a toolchain the runner does not
/// have by default — thread sanitizers, benchmark baselines — are deliberately
/// left out: their rules will fire, get a review date, and become a decision
/// somebody made rather than a step nobody reads.
fn workflow(languages: &[String], selected: &[&crate::catalog::Rule]) -> String {
    let mut out = String::from(
        "name: software factory\n\n\
         on:\n  pull_request:\n  push:\n    branches: [main]\n\n\
         permissions:\n  contents: read\n\n\
         jobs:\n  factory:\n    runs-on: ubuntu-latest\n    steps:\n      \
         - uses: actions/checkout@v4\n        with:\n          fetch-depth: 0\n      \
         - name: Install sf\n        run: cargo install --git https://github.com/nicolasmelo1/software-factory --locked\n      \
         # verify first: a check that stopped firing makes the run below\n      \
         # meaningless, and it is the cheaper failure to discover.\n      \
         - name: Prove the checks still fire\n        run: sf verify\n      \
         - name: Check the repository\n        run: sf check --changed origin/${{ github.base_ref || 'main' }}\n",
    );
    if !selected.iter().any(|r| r.layer == crate::catalog::Layer::L6) {
        return out;
    }
    out.push_str("\n  hazards:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n");
    out.push_str(
        "      - name: Committed secrets\n        uses: gitleaks/gitleaks-action@v2\n",
    );
    for language in languages {
        out.push_str(hazard_steps(language));
    }
    out
}

fn hazard_steps(language: &str) -> &'static str {
    match language {
        "python" => concat!(
            "      - uses: actions/setup-python@v5\n        with:\n          python-version: '3.12'\n",
            "      - run: pip install pip-audit bandit vulture\n",
            "      - name: Dependency vulnerabilities\n        run: pip-audit\n",
            "      - name: Insecure patterns\n        run: bandit -r . -c pyproject.toml\n",
            "      - name: Dead code\n        run: vulture .\n",
        ),
        "typescript" => concat!(
            "      - uses: actions/setup-node@v4\n        with:\n          node-version: '22'\n",
            "      - name: Dependency vulnerabilities\n        run: npm audit --audit-level=high\n",
            "      - name: Insecure patterns\n        run: npx --yes semgrep --config auto --error\n",
            "      - name: Dead code\n        run: npx --yes knip\n",
        ),
        "go" => concat!(
            "      - uses: actions/setup-go@v5\n        with:\n          go-version: stable\n",
            "      - name: Dependency vulnerabilities\n        run: go run golang.org/x/vuln/cmd/govulncheck@latest ./...\n",
            "      - name: Insecure patterns\n        run: go run github.com/securego/gosec/v2/cmd/gosec@latest ./...\n",
            "      - name: Dead code\n        run: go run honnef.co/go/tools/cmd/staticcheck@latest ./...\n",
            "      - name: Data races\n        run: go test -race ./...\n",
        ),
        "rust" => concat!(
            "      - name: Dependency vulnerabilities\n        uses: taiki-e/install-action@cargo-audit\n",
            "      - run: cargo audit\n",
            "      - name: Insecure patterns and dead code\n        run: cargo clippy --all-targets -- -D warnings -D dead_code\n",
        ),
        _ => "",
    }
}

const PRE_COMMIT: &str = "#!/bin/sh\n\
# Enable with: git config core.hooksPath .githooks\n\
set -eu\n\n\
# verify first: a check that no longer fires would make the run below\n\
# meaningless, and it is the cheaper failure to discover.\n\
sf verify\n\
sf check\n";
