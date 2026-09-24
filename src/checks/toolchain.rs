//! L6 — the hazard hunt actually runs.
//!
//! `sf` does not reimplement a vulnerability database, a secret scanner or a
//! race detector. Those tools exist, they are better than anything this could
//! contain, and they are different per language. What is missing in most
//! repositories is not the tool — it is the guarantee that the tool is still
//! wired in.
//!
//! So a rule names a *concern* (language-neutral) and a set of tools that
//! cover it (per language), and the check asserts that at least one of them
//! appears somewhere the repository actually executes: a CI workflow, a
//! Makefile, a task runner, a package script.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::policy::Options;
use crate::scan;
use anyhow::Result;

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    if opts.tools.is_empty() {
        return Ok(Vec::new());
    }
    let runners: Vec<String> = scan::select(ctx.files, &opts.scope, &opts.exclude)?
        .iter()
        .filter_map(|file| std::fs::read_to_string(&file.abs).ok())
        .map(|content| without_comment_lines(&content))
        .collect();
    let haystack = runners.join("\n");

    let mut findings = Vec::new();
    for language in &ctx.policy.project.languages {
        let Some(candidates) = opts.tools.get(language) else {
            // No tool is claimed to cover this concern in this language. That
            // is a statement about the ecosystem, not a violation.
            continue;
        };
        if candidates
            .iter()
            .any(|tool| haystack.contains(tool.as_str()))
        {
            continue;
        }
        findings.push(
            Finding::new(
                &rule.id,
                rule.severity,
                opts.scope.join(", "),
                language.clone(),
                format!("nothing in this repository runs a {language} tool for this hazard"),
            )
            .expected(format!("one of: {}", candidates.join(", ")))
            .actual("not found in any CI workflow or task runner".to_string()),
        );
    }
    Ok(findings)
}

/// A tool named in a comment is prose, not a wired-in guarantee. `sf init`
/// writes explanatory `#` comments naming tools beside the steps that run
/// them, so counting comment text let a freshly generated workflow satisfy
/// these rules by itself.
///
/// Every file in scope uses `#` for comments or none at all, so dropping
/// `#`-leading lines cannot hide a real `uses:`/`run:`/recipe invocation.
fn without_comment_lines(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::without_comment_lines;

    #[test]
    fn a_tool_named_only_in_a_comment_is_not_evidence() {
        let workflow = "\
  # Committed secrets: we run gitleaks on every push.\n\
  # (comment-only mention: nothing here actually invokes gitleaks)\n";
        let stripped = without_comment_lines(workflow);
        assert!(!stripped.contains("gitleaks"));
    }

    #[test]
    fn a_real_invocation_survives_the_comment_strip() {
        let workflow = "\
  # Committed secrets: scanned by gitleaks.\n\
  - uses: gitleaks/gitleaks-action@v2\n";
        let stripped = without_comment_lines(workflow);
        assert!(stripped.contains("gitleaks"));
    }

    /// The runtime-pin preflight shares a layer of vocabulary with this
    /// check and nothing else. A hazard rule keeps its kind, its key and
    /// every word of its output, pinned here so a change to one cannot leak
    /// into the other.
    #[test]
    fn a_hazard_rule_keeps_its_kind_and_its_output() {
        use crate::catalog::{Catalog, CheckKind};
        use crate::checks::{Ctx, run_all_with, runtime_pin};
        use crate::policy::Policy;
        use crate::ratchet::Ratchet;

        let root = runtime_pin::scratch(
            "toolchain",
            &[
                (
                    ".github/workflows/ci.yml",
                    "on: [push]\njobs:\n  test:\n    steps:\n      - run: pytest\n",
                ),
                (".nvmrc", "20.11.1\n"),
            ],
        );
        let policy: Policy = serde_yaml::from_str(
            "version: 1\nproject:\n  name: hazard\n  languages: [python]\nrules:\n  L6.SECRETS_ARE_SCANNED:\n    enabled: true\n  L2.RUNNING_TOOLCHAIN_MATCHES_THE_PIN:\n    enabled: true\n",
        )
        .expect("the policy parses");
        let catalog = Catalog::builtin().expect("the built-in catalog loads");
        let rule = catalog
            .get("L6.SECRETS_ARE_SCANNED")
            .expect("the rule ships");
        assert!(matches!(rule.check, CheckKind::Toolchain));
        let files = crate::scan::walk(&root, &policy).expect("the scratch repo scans");
        let ratchet = Ratchet::default();
        let ctx = Ctx {
            root: &root,
            policy: &policy,
            catalog: &catalog,
            files: &files,
            ratchet: &ratchet,
            changed: None,
            base: None,
            today: crate::clock::today(),
            allow_commands: false,
            overlay: None,
        };
        let answers = runtime_pin::Answers::printing(&[("node --version", "v20.11.1")]);
        let found = run_all_with(&ctx, &answers).expect("the run completes");
        assert_eq!(found.len(), 1, "{found:?}");
        let hazard = &found[0];
        assert_eq!(hazard.rule, "L6.SECRETS_ARE_SCANNED");
        assert_eq!(hazard.key, "python");
        assert_eq!(
            hazard.message,
            "nothing in this repository runs a python tool for this hazard"
        );
        assert_eq!(
            hazard.actual.as_deref(),
            Some("not found in any CI workflow or task runner")
        );
        assert!(
            hazard
                .expected
                .as_deref()
                .is_some_and(|e| e.starts_with("one of: "))
        );
    }
}
