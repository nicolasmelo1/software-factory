//! L1 — banned textual shapes.
//!
//! Deliberately line-based rather than AST-based: suppression comments and
//! escape hatches are lexical, they appear in every language, and a repo
//! should be able to add one without waiting for a grammar. The message on
//! each pattern is the point — it is the documentation the agent will
//! actually read, so it must name the alternative, not just the ban.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::policy::Options;
use crate::scan;
use anyhow::{Context, Result};
use regex::Regex;
use crate::digest;

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let selected = scan::select(ctx.files, &opts.scope, &opts.exclude)?;
    let compiled: Vec<(Regex, Option<Regex>, &str)> = opts
        .forbidden
        .iter()
        .map(|p| {
            let forbidden = Regex::new(&p.regex)
                .with_context(|| format!("rule {} has an invalid regex", rule.id))?;
            let unless = p.unless.as_deref().map(Regex::new).transpose()?;
            Ok((forbidden, unless, p.message.as_str()))
        })
        .collect::<Result<_>>()?;

    let mut findings = Vec::new();
    for file in selected {
        let Ok(source) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        for (number, line) in source.lines().enumerate() {
            for (forbidden, unless, message) in &compiled {
                if !forbidden.is_match(line) {
                    continue;
                }
                if unless.as_ref().is_some_and(|u| u.is_match(line)) {
                    continue;
                }
                // Key on the content, not the line number: moving a file's
                // imports around must not silently un-freeze the ratchet.
                let digest = &digest::hex(line.trim().as_bytes())[..12];
                findings.push(
                    Finding::new(
                        &rule.id,
                        rule.severity,
                        format!("{}:{}", file.rel, number + 1),
                        format!("{}:{digest}", file.rel),
                        message.to_string(),
                    )
                    .actual(line.trim().to_string()),
                );
            }
        }
    }
    Ok(findings)
}
