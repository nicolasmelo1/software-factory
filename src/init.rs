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
use crate::policy::{FIXTURES_DIR, POLICY_PATH, Policy};
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
}

pub fn run(root: &Path, catalog: &Catalog, opts: &InitOptions) -> Result<Vec<String>> {
    let policy_path = root.join(POLICY_PATH);
    if policy_path.exists() && !opts.force {
        bail!("{} already exists — pass --force to overwrite", policy_path.display());
    }

    let selected: Vec<&crate::catalog::Rule> = catalog
        .rules
        .values()
        .filter(|r| opts.layers.iter().any(|l| l == r.layer.as_str()))
        .collect();
    if selected.is_empty() {
        bail!("no rules match the requested layers");
    }

    let mut written = Vec::new();
    write(root, POLICY_PATH, &policy_document(opts, &selected), &mut written)?;
    write(root, "docs/rules.md", &rules_document(opts, &selected), &mut written)?;
    write_cadence_files(root, &selected, &mut written)?;
    write_automation(root, &mut written)?;
    write_fixtures(root, &selected, &mut written)?;
    Ok(written)
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
    write(root, ".github/workflows/software-factory.yml", WORKFLOW, written)?;
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

fn write(root: &Path, rel: &str, body: &str, written: &mut Vec<String>) -> Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body)?;
    written.push(rel.to_string());
    Ok(())
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
        today: clock::today(),
    };
    let review_by = clock::plus_months(&ctx.today, months);
    let mut ratchet = Ratchet::default();
    let mut total = 0;
    for (id, rule) in &catalog.rules {
        if policy.enabled(id).is_none() || rule.ratchet == RatchetPolicy::None {
            continue;
        }
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
        today: clock::today(),
    };
    let mut written = Vec::new();
    for (id, rule) in &catalog.rules {
        if policy.enabled(id).is_none() {
            continue;
        }
        if !matches!(rule.check, crate::catalog::CheckKind::Lock) {
            continue;
        }
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

fn policy_document(opts: &InitOptions, selected: &[&crate::catalog::Rule]) -> String {
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
    out.push_str("  exclude: []\n\ndocs:\n  scan:\n    - \"docs/**/*.md\"\n    - \"*.md\"\n\n");
    out.push_str(
        "# A gate activates from the paths a change touches, never from a label\n\
         # or a sentence in a pull request. See L3.GATE_HAS_FRESH_EVIDENCE.\n\
         gates: {}\n\nrules:\n",
    );
    for rule in selected {
        out.push_str(&format!("  # {} — {}\n", rule.layer.as_str(), rule.title));
        out.push_str(&format!("  {}:\n    enabled: true\n", rule.id));
        if matches!(rule.check, crate::catalog::CheckKind::Lock) {
            out.push_str("    options:\n      # Nothing is locked until you say what is generated.\n      scope: []\n");
        }
    }
    out
}

fn rules_document(opts: &InitOptions, selected: &[&crate::catalog::Rule]) -> String {
    let mut out = format!(
        "# Rules {} enforces\n\n\
         Generated by `sf init`. Every rule below is enforced in CI; this document\n\
         is the other half of the pair — the half that says why. Replace the\n\
         generic reasoning with this repository's own decision as it acquires one,\n\
         but do not delete a rule's section while the rule is enabled:\n\
         `L4.EVERY_RULE_HAS_A_WHY` fails when enforcement and prose come apart.\n",
        opts.name
    );
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

const WORKFLOW: &str = "name: software factory\n\n\
on:\n  pull_request:\n  push:\n    branches: [main]\n\n\
permissions:\n  contents: read\n\n\
jobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n      \
- uses: actions/checkout@v4\n        with:\n          fetch-depth: 0\n      \
- name: Install sf\n        run: cargo install --git https://github.com/nicolasmelo1/software-factory --locked\n      \
- name: Prove the checks still fire\n        run: sf verify\n      \
- name: Check the repository\n        run: sf check --changed origin/${{ github.base_ref || 'main' }}\n";

const PRE_COMMIT: &str = "#!/bin/sh\n\
# Enable with: git config core.hooksPath .githooks\n\
set -eu\n\n\
# verify first: a check that no longer fires would make the run below\n\
# meaningless, and it is the cheaper failure to discover.\n\
sf verify\n\
sf check\n";
