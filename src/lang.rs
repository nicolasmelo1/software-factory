//! Language registry.
//!
//! A language contributes three things and nothing else: a tree-sitter
//! grammar, the node kinds that open a function, and the node kinds that
//! branch. Every rule in the catalog is written against those, which is why
//! the catalog itself stays language-neutral.

use anyhow::{Result, anyhow};
use std::path::Path;
use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lang {
    Python,
    TypeScript,
    Tsx,
    Go,
    Rust,
    Ruby,
}

impl Lang {
    pub fn name(&self) -> &'static str {
        match self {
            // tsx shares the typescript rule surface: a rule written for
            // `typescript` applies to `.tsx` too, or half a React repo is unchecked.
            Lang::Python => "python",
            Lang::TypeScript | Lang::Tsx => "typescript",
            Lang::Go => "go",
            Lang::Rust => "rust",
            Lang::Ruby => "ruby",
        }
    }

    pub fn grammar(&self) -> Language {
        match self {
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Go => tree_sitter_go::LANGUAGE.into(),
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        }
    }

    pub fn from_path(path: &Path) -> Option<Lang> {
        match path.extension()?.to_str()? {
            "py" | "pyi" => Some(Lang::Python),
            "ts" | "mts" | "cts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::Tsx),
            "go" => Some(Lang::Go),
            "rs" => Some(Lang::Rust),
            "rb" | "rake" | "gemspec" | "ru" => Some(Lang::Ruby),
            _ => None,
        }
    }


    pub fn from_name(name: &str) -> Result<Lang> {
        match name {
            "python" => Ok(Lang::Python),
            "typescript" => Ok(Lang::TypeScript),
            "tsx" => Ok(Lang::Tsx),
            "go" => Ok(Lang::Go),
            "rust" => Ok(Lang::Rust),
            "ruby" => Ok(Lang::Ruby),
            other => Err(anyhow!("unknown language {other:?}")),
        }
    }

    /// Node kinds that open a callable body, for the complexity ceiling.
    pub fn function_kinds(&self) -> &'static [&'static str] {
        match self {
            Lang::Python => &["function_definition"],
            Lang::TypeScript | Lang::Tsx => &[
                "function_declaration",
                "function_expression",
                "method_definition",
                "arrow_function",
                "generator_function_declaration",
            ],
            Lang::Go => &["function_declaration", "method_declaration", "func_literal"],
            Lang::Rust => &["function_item", "closure_expression"],
            Lang::Ruby => &["method", "singleton_method"],
        }
    }

    /// Node kinds that add one independent path through a function.
    pub fn branch_kinds(&self) -> &'static [&'static str] {
        match self {
            Lang::Python => &[
                "if_statement",
                "elif_clause",
                "for_statement",
                "while_statement",
                "except_clause",
                "conditional_expression",
                "case_clause",
                "assert_statement",
                "list_comprehension",
                "dictionary_comprehension",
                "set_comprehension",
                "generator_expression",
            ],
            Lang::TypeScript | Lang::Tsx => &[
                "if_statement",
                "for_statement",
                "for_in_statement",
                "while_statement",
                "do_statement",
                "catch_clause",
                "switch_case",
                "ternary_expression",
            ],
            Lang::Go => &[
                "if_statement",
                "for_statement",
                "expression_case",
                "type_case",
                "communication_case",
                "select_statement",
            ],
            Lang::Rust => &[
                "if_expression",
                "for_expression",
                "while_expression",
                "loop_expression",
                "match_arm",
                "let_condition",
                // `?` is deliberately absent. It is an early return, so a
                // strict McCabe count includes it — but in Rust it is what you
                // write *instead of* branching, and counting it penalises the
                // idiom while rewarding `.unwrap()`, which another rule bans.
                // A rule that pushes toward worse code is miscalibrated.
            ],
            Lang::Ruby => &[
                "if",
                "elsif",
                "unless",
                "while",
                "until",
                "for",
                "when",
                "in_clause",
                "rescue",
                "conditional",
                "if_modifier",
                "unless_modifier",
                "while_modifier",
                "until_modifier",
                "rescue_modifier",
            ],
        }
    }

    /// Kinds whose operator must be inspected: `a && b` is a branch, `a + b` is not.
    pub fn boolean_operator_kinds(&self) -> &'static [&'static str] {
        match self {
            Lang::Python => &["boolean_operator"],
            Lang::TypeScript | Lang::Tsx => &["binary_expression"],
            Lang::Go => &["binary_expression"],
            Lang::Rust => &["binary_expression"],
            Lang::Ruby => &["binary"],
        }
    }

    pub fn boolean_operators(&self) -> &'static [&'static str] {
        match self {
            Lang::Python => &["and", "or"],
            Lang::TypeScript | Lang::Tsx => &["&&", "||", "??"],
            Lang::Go => &["&&", "||"],
            Lang::Rust => &["&&", "||"],
            Lang::Ruby => &["&&", "||", "and", "or"],
        }
    }
}
