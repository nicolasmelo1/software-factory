//! L6 — containment hazards.
//!
//! Static analysis cannot decide whether a program deadlocks; that is
//! undecidable in general, and no amount of tooling changes it. What it can do
//! is forbid the shapes that cause the deadlocks and starvation people
//! actually ship: blocking while holding a lock, awaiting while holding a
//! synchronous lock, and taking a second lock inside the first.
//!
//! So this engine answers one question — does `inner` appear anywhere inside
//! `outer`? — and the catalog supplies what those two are per language.

use super::Ctx;
use crate::catalog::{NestedQuery, Rule};
use crate::finding::Finding;
use crate::lang::Lang;
use crate::policy::Options;
use crate::scan;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

pub fn run(
    rule: &Rule,
    opts: &Options,
    languages: &BTreeMap<String, NestedQuery>,
    ctx: &Ctx,
) -> Result<Vec<Finding>> {
    let threshold = opts.min_inner.unwrap_or(1);
    let mut findings = Vec::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Some(lang) = Lang::from_path(&file.abs) else {
            continue;
        };
        if !ctx.policy.project.languages.iter().any(|l| l == lang.name()) {
            continue;
        }
        let Some(spec) = languages.get(lang.name()) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        findings.extend(scan_file(rule, lang, spec, &file.rel, &source, threshold)?);
    }
    Ok(findings)
}

fn scan_file(
    rule: &Rule,
    lang: Lang,
    spec: &NestedQuery,
    rel: &str,
    source: &str,
    threshold: usize,
) -> Result<Vec<Finding>> {
    let grammar = lang.grammar();
    let outer = Query::new(&grammar, &spec.outer)
        .with_context(|| format!("rule {} has an invalid {} outer query", rule.id, lang.name()))?;
    let inner = Query::new(&grammar, &spec.inner)
        .with_context(|| format!("rule {} has an invalid {} inner query", rule.id, lang.name()))?;
    let mut parser = Parser::new();
    parser.set_language(&grammar)?;
    let Some(tree) = parser.parse(source, None) else {
        return Ok(Vec::new());
    };

    let outer_names = outer.capture_names();
    let bytes = source.as_bytes();
    let mut findings = Vec::new();
    let mut outer_cursor = QueryCursor::new();
    let mut outer_matches = outer_cursor.matches(&outer, tree.root_node(), bytes);
    while let Some(m) = outer_matches.next() {
        for capture in m.captures {
            if outer_names[capture.index as usize] == "target" {
                findings.extend(inside(rule, &inner, capture.node, rel, bytes, threshold));
            }
        }
    }
    Ok(findings)
}

/// Findings for `inner` matches within one guarded region. The cursor is
/// rooted at the region, so tree-sitter only walks that subtree — which is
/// exactly what "inside the lock" means.
fn inside(
    rule: &Rule,
    inner: &Query,
    region: Node,
    rel: &str,
    bytes: &[u8],
    threshold: usize,
) -> Vec<Finding> {
    let names = inner.capture_names();
    let opened = region.start_position().row + 1;
    let mut findings = Vec::new();
    let mut seen = 0;
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(inner, region, bytes);
    while let Some(hit) = matches.next() {
        for found in hit.captures {
            // The region matches its own inner query. That is not a nesting.
            if names[found.index as usize] != "inner" || found.node.id() == region.id() {
                continue;
            }
            seen += 1;
            if seen < threshold {
                continue;
            }
            findings.push(finding_for(rule, found.node, rel, bytes, opened, seen, threshold));
        }
    }
    findings
}

fn finding_for(
    rule: &Rule,
    node: Node,
    rel: &str,
    bytes: &[u8],
    opened: usize,
    seen: usize,
    threshold: usize,
) -> Finding {
    let line = node.start_position().row + 1;
    let text = node
        .utf8_text(bytes)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let message = if threshold > 1 {
        format!("`{}` is acquisition #{seen} in the scope starting at line {opened}", truncate(&text, 60))
    } else {
        format!("`{}` appears inside the guarded region starting at line {opened}", truncate(&text, 60))
    };
    Finding::new(
        &rule.id,
        rule.severity,
        format!("{rel}:{line}"),
        format!("{rel}:{}", &crate::digest::hex(text.as_bytes())[..12]),
        message,
    )
    .actual(text)
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    format!("{}…", text.chars().take(max).collect::<String>())
}
