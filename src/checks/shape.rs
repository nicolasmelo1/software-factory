//! L0 — structural placement.
//!
//! A rule says which AST nodes it cares about (a tree-sitter query per
//! language) and where those nodes are allowed to live (globs). The engine
//! knows nothing about controllers, repositories or exceptions: that
//! vocabulary lives entirely in the catalog, which is what lets one rule mean
//! the same thing in Python, TypeScript and Go.

use super::Ctx;
use crate::catalog::{LangQuery, Rule};
use crate::finding::Finding;
use crate::lang::Lang;
use crate::policy::Options;
use crate::scan;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

struct Match {
    file: String,
    name: String,
    line: usize,
}

pub fn run(
    rule: &Rule,
    opts: &Options,
    languages: &BTreeMap<String, LangQuery>,
    ctx: &Ctx,
) -> Result<Vec<Finding>> {
    let matches = collect(rule, opts, languages, ctx)?;
    let mut findings = Vec::new();
    findings.extend(placement(rule, opts, &matches)?);
    findings.extend(density(rule, opts, &matches));
    Ok(findings)
}

/// Every node in scope that the rule's queries match.
fn collect(
    rule: &Rule,
    opts: &Options,
    languages: &BTreeMap<String, LangQuery>,
    ctx: &Ctx,
) -> Result<Vec<Match>> {
    let mut matches = Vec::new();
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
        // Not utf-8: nothing this rule can say about it.
        let Ok(source) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        matches.extend(matches_in(rule, lang, spec, &file.rel, &source)?);
    }
    Ok(matches)
}

/// Every match of `query` that `unless` does not cancel.
///
/// The cancelling query matches the same shape plus whatever makes it
/// acceptable, so the two agree on the `@target` line and the difference is a
/// line-set subtraction. Kept separate from `collect` so it can be tested
/// against the catalog's real queries without a repository around it.
fn matches_in(
    rule: &Rule,
    lang: Lang,
    spec: &LangQuery,
    rel: &str,
    source: &str,
) -> Result<Vec<Match>> {
    let found = query_file(rule, lang, &spec.query, rel, source)?;
    let Some(unless) = &spec.unless else {
        return Ok(found);
    };
    let accepted: BTreeSet<usize> =
        query_file(rule, lang, unless, rel, source)?.into_iter().map(|m| m.line).collect();
    Ok(found.into_iter().filter(|m| !accepted.contains(&m.line)).collect())
}

fn query_file(
    rule: &Rule,
    lang: Lang,
    query_source: &str,
    rel: &str,
    source: &str,
) -> Result<Vec<Match>> {
    let grammar = lang.grammar();
    let query = Query::new(&grammar, query_source)
        .with_context(|| format!("rule {} has an invalid {} query", rule.id, lang.name()))?;
    let mut parser = Parser::new();
    parser.set_language(&grammar)?;
    let Some(tree) = parser.parse(source, None) else {
        return Ok(Vec::new());
    };
    let names = query.capture_names();
    let mut out = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = iter.next() {
        let mut name = None;
        let mut line = None;
        for capture in m.captures {
            match names[capture.index as usize] {
                "name" => name = capture.node.utf8_text(source.as_bytes()).ok().map(trim_quotes),
                "target" => line = Some(capture.node.start_position().row + 1),
                _ => {}
            }
        }
        let line = line.unwrap_or(0);
        out.push(Match {
            file: rel.to_string(),
            name: name.unwrap_or_else(|| format!("L{line}")),
            line,
        });
    }
    Ok(out)
}

/// Where a match is allowed — or forbidden — to live.
fn placement(rule: &Rule, opts: &Options, matches: &[Match]) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    if !opts.must_live_in.is_empty() {
        let allowed = scan::globs(&opts.must_live_in)?;
        for m in matches.iter().filter(|m| !allowed.is_match(&m.file)) {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    format!("{}:{}", m.file, m.line),
                    format!("{}:{}", m.file, m.name),
                    format!("`{}` is defined outside its allowed location", m.name),
                )
                .expected(opts.must_live_in.join(", "))
                .actual(m.file.clone()),
            );
        }
    }
    if !opts.must_not_live_in.is_empty() {
        let forbidden = scan::globs(&opts.must_not_live_in)?;
        for m in matches.iter().filter(|m| forbidden.is_match(&m.file)) {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    format!("{}:{}", m.file, m.line),
                    format!("{}:{}", m.file, m.name),
                    format!("`{}` appears where this rule forbids it", m.name),
                )
                .expected(format!("not under {}", opts.must_not_live_in.join(", ")))
                .actual(m.file.clone()),
            );
        }
    }
    Ok(findings)
}

/// How many matches one file may hold.
fn density(rule: &Rule, opts: &Options, matches: &[Match]) -> Vec<Finding> {
    let Some(max) = opts.max_per_file else {
        return Vec::new();
    };
    let mut per_file: BTreeMap<&str, usize> = BTreeMap::new();
    for m in matches {
        *per_file.entry(m.file.as_str()).or_default() += 1;
    }
    per_file
        .into_iter()
        .filter(|(_, count)| *count > max)
        .map(|(file, count)| {
            Finding::new(
                &rule.id,
                rule.severity,
                file.to_string(),
                file.to_string(),
                format!("{count} declarations in one file, at most {max} allowed"),
            )
            .expected(max.to_string())
            .actual(count.to_string())
        })
        .collect()
}

fn trim_quotes(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string()
}

#[cfg(test)]
mod cancelling_query {
    use super::*;
    use crate::catalog::{Catalog, CheckKind};

    const TESTS: &str = "describe(\"refunds\", () => {\n  \
        it.skip(\"is idempotent\", () => {});\n\n  \
        // Flaky against the sandbox gateway; back when TICKET-4711 lands.\n  \
        it.skip(\"settles twice\", () => {});\n});\n";

    fn typescript(rule_id: &str) -> (crate::catalog::Rule, LangQuery) {
        let catalog = Catalog::builtin().expect("the builtin catalog loads");
        let rule = catalog.get(rule_id).expect("the rule ships in the catalog").clone();
        let CheckKind::Shape { languages } = &rule.check else {
            panic!("{rule_id} is not a shape rule");
        };
        let spec = languages.get("typescript").expect("the rule ships a typescript query").clone();
        (rule, spec)
    }

    /// The pair that makes `unless` meaningful: the query has to see both
    /// skips, or the filter below proves nothing.
    #[test]
    fn the_query_alone_matches_every_skip() {
        let (rule, spec) = typescript("L1.SKIPPED_TESTS_STATE_A_REASON");
        let found = query_file(&rule, Lang::TypeScript, &spec.query, "billing.test.ts", TESTS)
            .expect("the typescript query is valid");
        assert_eq!(found.iter().map(|m| m.line).collect::<Vec<_>>(), vec![2, 5]);
    }

    #[test]
    fn a_comment_above_the_skip_cancels_it() {
        let (rule, spec) = typescript("L1.SKIPPED_TESTS_STATE_A_REASON");
        let found = matches_in(&rule, Lang::TypeScript, &spec, "billing.test.ts", TESTS)
            .expect("both typescript queries are valid");
        assert_eq!(
            found.iter().map(|m| m.line).collect::<Vec<_>>(),
            vec![2],
            "the documented skip on line 5 should be cancelled, the bare one on line 2 should not"
        );
    }
}
