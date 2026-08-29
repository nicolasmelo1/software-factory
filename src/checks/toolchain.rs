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

/// A mention of a tool inside a comment is prose, not a wired-in guarantee.
/// `sf init` itself writes explanatory `#` comments naming the tools next to
/// the steps that run them, so counting comment text let a freshly generated
/// workflow satisfy these rules all by itself. Every file in scope here
/// (YAML workflows, Makefile, justfile, Taskfile, .gitlab-ci.yml,
/// .pre-commit-config.yaml, package.json) uses `#` for comments or none at
/// all, so dropping `#`-leading lines cannot hide a real invocation — those
/// are `uses:`/`run:`/recipe lines, which never start with `#`.
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
}
