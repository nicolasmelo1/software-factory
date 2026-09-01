//! L1 — the comment-block ceiling.
//!
//! A contiguous run of whole-line comments is measured and compared against a
//! ceiling. Deliberately line-based rather than grammar-based: the rule has to
//! reach YAML, TOML and shell, which carry the longest comment blocks in a
//! typical repository and which no AST-driven linter covers. A marker per
//! extension is the only language knowledge needed, and it is knowledge the
//! rule can hold for a file type that has no parser here.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::policy::Options;
use crate::scan;
use anyhow::Result;
use std::path::Path;

/// The line-comment marker for extensions this rule understands. A file type
/// absent from this table is skipped rather than guessed at: inventing a
/// marker would produce findings nobody can act on.
const MARKERS: &[(&str, &str)] = &[
    ("py", "#"),
    ("pyi", "#"),
    ("yaml", "#"),
    ("yml", "#"),
    ("toml", "#"),
    ("sh", "#"),
    ("bash", "#"),
    ("rb", "#"),
    ("ts", "//"),
    ("tsx", "//"),
    ("js", "//"),
    ("jsx", "//"),
    ("mjs", "//"),
    ("cjs", "//"),
    ("go", "//"),
    ("rs", "//"),
];

fn marker_for(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    MARKERS
        .iter()
        .find(|(name, _)| *name == ext)
        .map(|(_, marker)| *marker)
}

/// A shebang is not a comment, and a divider rule of `####` carries no prose.
/// Neither should count toward a budget meant to measure explanation.
fn is_prose_comment(line: &str, marker: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(marker) {
        return false;
    }
    if trimmed.starts_with("#!") {
        return false;
    }
    let body = trimmed.trim_start_matches(marker).trim();
    !body.is_empty() && body.chars().any(char::is_alphanumeric)
}

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let max = opts.max.unwrap_or(6);
    let selected = scan::select(ctx.files, &opts.scope, &opts.exclude)?;
    let mut findings = Vec::new();

    for file in selected {
        let Some(marker) = marker_for(&file.abs) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        for (start, len) in blocks(&source, marker) {
            if len > max {
                findings.push(
                    Finding::new(
                        &rule.id,
                        rule.severity,
                        format!("{}:{}", file.rel, start),
                        format!("{}:{}", file.rel, start),
                        format!(
                            "comment block runs {len} lines, ceiling is {max}"
                        ),
                    )
                    .expected(format!("at most {max} lines"))
                    .actual(format!("{len} lines")),
                );
            }
        }
    }
    Ok(findings)
}

/// Contiguous runs of prose comment lines, as `(1-based start line, length)`.
///
/// A blank line ends a block. That is the point: splitting an explanation into
/// paragraphs is the cheapest honest way to stay under the ceiling, and a rule
/// that counted across blank lines would forbid the fix it is asking for.
fn blocks(source: &str, marker: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut len = 0usize;
    for (index, line) in source.lines().enumerate() {
        if is_prose_comment(line, marker) {
            if len == 0 {
                start = index + 1;
            }
            len += 1;
        } else if len > 0 {
            out.push((start, len));
            len = 0;
        }
    }
    if len > 0 {
        out.push((start, len));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_a_contiguous_run() {
        let src = "# one\n# two\n# three\ncode()\n";
        assert_eq!(blocks(src, "#"), vec![(1, 3)]);
    }

    #[test]
    fn a_blank_line_ends_a_block() {
        let src = "# one\n# two\n\n# three\n";
        assert_eq!(blocks(src, "#"), vec![(1, 2), (4, 1)]);
    }

    #[test]
    fn a_shebang_is_not_a_comment() {
        let src = "#!/usr/bin/env bash\n# one\n";
        assert_eq!(blocks(src, "#"), vec![(2, 1)]);
    }

    #[test]
    fn a_divider_rule_carries_no_prose() {
        let src = "# ------\n# real\n# ======\n";
        assert_eq!(blocks(src, "#"), vec![(2, 1)]);
    }

    #[test]
    fn trailing_comments_are_not_whole_line_comments() {
        let src = "code()  # explains the line\nmore()\n";
        assert!(blocks(src, "#").is_empty());
    }

    #[test]
    fn slash_marker_reads_typescript() {
        let src = "// one\n// two\nconst x = 1;\n";
        assert_eq!(blocks(src, "//"), vec![(1, 2)]);
    }

    #[test]
    fn a_url_inside_code_is_not_a_comment() {
        let src = "const u = \"https://example.com\";\n";
        assert!(blocks(src, "//").is_empty());
    }
}
