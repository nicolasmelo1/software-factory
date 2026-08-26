//! L1 — the cyclomatic ceiling.
//!
//! One independent path plus one per branching construct, attributed to the
//! function that owns it: a nested closure gets its own budget rather than
//! inflating its parent's.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::lang::Lang;
use crate::policy::Options;
use crate::scan;
use anyhow::Result;
use tree_sitter::{Node, Parser};

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let max = opts.max.unwrap_or(12);
    let selected = scan::select(ctx.files, &opts.scope, &opts.exclude)?;
    let mut findings = Vec::new();

    for file in selected {
        let Some(lang) = Lang::from_path(&file.abs) else {
            continue;
        };
        if !ctx.policy.project.languages.iter().any(|l| l == lang.name()) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        let grammar = lang.grammar();
        let mut parser = Parser::new();
        parser.set_language(&grammar)?;
        let Some(tree) = parser.parse(&source, None) else {
            continue;
        };
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if lang.function_kinds().contains(&node.kind()) {
                let score = complexity(node, lang, source.as_bytes());
                if score > max {
                    let name = function_name(node, source.as_bytes())
                        .unwrap_or_else(|| format!("anonymous@{}", node.start_position().row + 1));
                    findings.push(
                        Finding::new(
                            &rule.id,
                            rule.severity,
                            format!("{}:{}", file.rel, node.start_position().row + 1),
                            format!("{}:{}", file.rel, name),
                            format!("`{name}` has {score} independent paths, ceiling is {max}"),
                        )
                        .expected(max.to_string())
                        .actual(score.to_string()),
                    );
                }
            }
            // Named children only: some grammars (Ruby) name a construct
            // identically to its own opening keyword token, e.g. the named
            // `if` node contains an anonymous `if` token as a child. Walking
            // anonymous nodes would match that token against the same
            // `branch_kinds` entry and double-count the branch it opens.
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                stack.push(child);
            }
        }
    }
    Ok(findings)
}

fn function_name(node: Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")?
        .utf8_text(source)
        .ok()
        .map(str::to_string)
}

/// Count branches under `node`, stopping at nested function boundaries.
fn complexity(node: Node, lang: Lang, source: &[u8]) -> usize {
    let mut score = 1;
    let mut stack: Vec<Node> = Vec::new();
    let mut cursor = node.walk();
    stack.extend(node.named_children(&mut cursor));
    while let Some(current) = stack.pop() {
        if lang.function_kinds().contains(&current.kind()) {
            continue; // its own budget
        }
        if lang.branch_kinds().contains(&current.kind()) {
            score += 1;
        } else if lang.boolean_operator_kinds().contains(&current.kind()) {
            let operator = current
                .child_by_field_name("operator")
                .and_then(|n| n.utf8_text(source).ok())
                .unwrap_or("");
            if lang.boolean_operators().contains(&operator) {
                score += 1;
            }
        }
        let mut inner = current.walk();
        stack.extend(current.named_children(&mut inner));
    }
    score
}
