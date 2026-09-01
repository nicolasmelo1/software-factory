//! L1 — the floor under the grain.
//!
//! Two queries per language and one comparison the queries cannot make. The
//! import query reads this file's own alias map, local name to upstream name,
//! so nothing here resolves a symbol across files. The forward query finds a
//! function whose only statement is one call. A finding is where the two meet:
//! the callee is an import whose upstream name is the wrapper's own name.

use super::Ctx;
use crate::catalog::{ForwarderQuery, Rule};
use crate::finding::Finding;
use crate::lang::Lang;
use crate::policy::Options;
use crate::scan;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

/// One query match, as the captured text per capture name plus the line the
/// first capture sits on.
struct Row {
    captured: HashMap<String, String>,
    line: usize,
}

pub fn run(
    rule: &Rule,
    opts: &Options,
    languages: &BTreeMap<String, ForwarderQuery>,
    ctx: &Ctx,
) -> Result<Vec<Finding>> {
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
        // Not utf-8: nothing this rule can say about it.
        let Ok(source) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        findings.extend(in_file(rule, lang, spec, &file.rel, &source)?);
    }
    Ok(findings)
}

/// Kept free of the walk so it can be tested against the catalog's real
/// queries without a repository around it.
fn in_file(
    rule: &Rule,
    lang: Lang,
    spec: &ForwarderQuery,
    rel: &str,
    source: &str,
) -> Result<Vec<Finding>> {
    let aliases = alias_map(rule, lang, &spec.import, source)?;
    if aliases.is_empty() {
        return Ok(Vec::new());
    }
    let mut findings = Vec::new();
    for row in rows(rule, lang, &spec.forward, source)? {
        let (Some(name), Some(callee)) = (row.captured.get("name"), row.captured.get("callee"))
        else {
            continue;
        };
        let Some(upstream) = aliases.get(callee) else {
            continue;
        };
        if upstream != name {
            continue;
        }
        findings.push(
            Finding::new(
                &rule.id,
                rule.severity,
                format!("{rel}:{}", row.line),
                format!("{rel}:{name}"),
                format!(
                    "`{name}` forwards to `{callee}`, which is `{upstream}` imported under another name"
                ),
            )
            .expected("a name a reader learns something from, or no wrapper".to_string())
            .actual(format!("a hop from `{name}` to `{name}`")),
        );
    }
    Ok(findings)
}

/// Local name to upstream name, for the aliased imports in one file.
///
/// Only aliased imports can produce this shape at all: without the alias the
/// wrapper and the import collide on one name, which every language here
/// either rejects or resolves into a wrapper that calls itself.
fn alias_map(
    rule: &Rule,
    lang: Lang,
    query: &str,
    source: &str,
) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for row in rows(rule, lang, query, source)? {
        let (Some(original), Some(local)) =
            (row.captured.get("original"), row.captured.get("local"))
        else {
            continue;
        };
        map.insert(local.clone(), original.clone());
    }
    Ok(map)
}

fn rows(rule: &Rule, lang: Lang, query_source: &str, source: &str) -> Result<Vec<Row>> {
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
        let mut captured = HashMap::new();
        let mut line = 0;
        for capture in m.captures {
            let Ok(text) = capture.node.utf8_text(source.as_bytes()) else {
                continue;
            };
            if line == 0 {
                line = capture.node.start_position().row + 1;
            }
            captured.insert(names[capture.index as usize].to_string(), text.to_string());
        }
        out.push(Row { captured, line });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::CheckKind;

    fn rule() -> Rule {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("catalog/L1/indirection-earns-its-name.yaml");
        let text = std::fs::read_to_string(path).expect("the rule ships in the catalog");
        serde_yaml::from_str(&text).expect("the rule parses")
    }

    fn spec(language: &str) -> ForwarderQuery {
        match rule().check {
            CheckKind::Forwarder { languages } => {
                languages.get(language).expect("the language has a query").clone()
            }
            _ => panic!("the rule is a forwarder rule"),
        }
    }

    fn findings(language: &str, lang: Lang, source: &str) -> Vec<Finding> {
        in_file(&rule(), lang, &spec(language), "src/service.x", source)
            .expect("the queries compile")
    }

    #[test]
    fn typescript_forwarder_under_the_imported_name() {
        let src = "import { insertCompression as insertCompressionDb } from \"@db\";\n\
                   export async function insertCompression(data: Data) {\n\
                   \x20 return insertCompressionDb(data);\n\
                   }\n";
        let found = findings("typescript", Lang::TypeScript, src);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].location, "src/service.x:2");
    }

    #[test]
    fn typescript_await_is_the_same_hop() {
        let src = "import { updateAccount as updateAccountDb } from \"@db\";\n\
                   export async function updateAccount(id: number) {\n\
                   \x20 return await updateAccountDb(id);\n\
                   }\n";
        assert_eq!(findings("typescript", Lang::TypeScript, src).len(), 1);
    }

    #[test]
    fn a_wrapper_that_renames_is_silent() {
        let src = "import { queryCompanyRecords } from \"@db\";\n\
                   export async function getCompanies(id: number) {\n\
                   \x20 return queryCompanyRecords(id);\n\
                   }\n";
        assert!(findings("typescript", Lang::TypeScript, src).is_empty());
    }

    #[test]
    fn a_body_that_does_more_than_forward_is_silent() {
        let src = "import { insertCompression as insertCompressionDb } from \"@db\";\n\
                   export async function insertCompression(data: Data) {\n\
                   \x20 const checked = validate(data);\n\
                   \x20 return insertCompressionDb(checked);\n\
                   }\n";
        assert!(findings("typescript", Lang::TypeScript, src).is_empty());
    }

    #[test]
    fn rust_reads_a_use_as_clause() {
        let src = "use crate::db::price as price_row;\n\
                   pub fn price(order: &Order) -> Money {\n\
                   \x20   price_row(order)\n\
                   }\n";
        let found = findings("rust", Lang::Rust, src);
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn python_reads_an_aliased_import() {
        let src = "from db import price as _price\n\n\
                   def price(order):\n\
                   \x20   return _price(order)\n";
        assert_eq!(findings("python", Lang::Python, src).len(), 1);
    }

    #[test]
    fn a_named_accessor_over_an_expression_is_silent() {
        let src = "use std::fs::read as read_file;\n\
                   pub fn file(path: &Path) -> Result<String> {\n\
                   \x20   read_file(path)\n\
                   }\n";
        assert!(findings("rust", Lang::Rust, src).is_empty());
    }
}
