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
        .collect();
    let haystack = runners.join("\n");

    let mut findings = Vec::new();
    for language in &ctx.policy.project.languages {
        let Some(candidates) = opts.tools.get(language) else {
            // No tool is claimed to cover this concern in this language. That
            // is a statement about the ecosystem, not a violation.
            continue;
        };
        if candidates.iter().any(|tool| haystack.contains(tool.as_str())) {
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
