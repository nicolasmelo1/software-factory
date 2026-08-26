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
use globset::GlobSet;

/// A line that is nothing but a comment. Deliberately prefix-based rather
/// than parsed: this engine runs on files no grammar covers, and a trailing
/// comment after real code is worth flagging anyway.
fn is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    ["//", "#", "*", "/*", "--", "\"\"\"", "'''"].iter().any(|marker| trimmed.starts_with(marker))
}

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let selected = scan::select(ctx.files, &opts.scope, &opts.exclude)?;

    struct Compiled<'a> {
        forbidden: Regex,
        unless: Option<Regex>,
        message: &'a str,
        // Per-pattern narrowing, one level under the rule's own scope/exclude.
        scope: Option<GlobSet>,
        exclude: Option<GlobSet>,
    }

    let compiled: Vec<Compiled> = opts
        .forbidden
        .iter()
        .map(|p| {
            let forbidden = Regex::new(&p.regex)
                .with_context(|| format!("rule {} has an invalid regex", rule.id))?;
            let unless = p.unless.as_deref().map(Regex::new).transpose()?;
            let scope = if p.scope.is_empty() { None } else { Some(scan::globs(&p.scope)?) };
            let exclude = if p.exclude.is_empty() { None } else { Some(scan::globs(&p.exclude)?) };
            Ok(Compiled { forbidden, unless, message: p.message.as_str(), scope, exclude })
        })
        .collect::<Result<_>>()?;

    let mut findings = Vec::new();
    for file in selected {
        let Ok(source) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        // Narrow once per file, not once per line: a pattern's scope/exclude
        // depends only on the path.
        let active: Vec<&Compiled> = compiled
            .iter()
            .filter(|p| p.scope.as_ref().is_none_or(|g| g.is_match(&file.rel)))
            .filter(|p| !p.exclude.as_ref().is_some_and(|g| g.is_match(&file.rel)))
            .collect();
        if active.is_empty() {
            continue;
        }
        for (number, line) in source.lines().enumerate() {
            if opts.ignore_comment_lines && is_comment(line) {
                continue;
            }
            for pattern in &active {
                if !pattern.forbidden.is_match(line) {
                    continue;
                }
                if pattern.unless.as_ref().is_some_and(|u| u.is_match(line)) {
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
                        pattern.message.to_string(),
                    )
                    .actual(line.trim().to_string()),
                );
            }
        }
    }
    Ok(findings)
}

/// The bug this file exists to fix: `L1.NO_UNTYPED_ESCAPE_HATCH`'s TypeScript
/// `any` pattern used to have no per-language restriction, so it also read
/// Ruby once commit #16 put `**/*.rb` in the rule's scope. `:\s*any\b`
/// matches the leading `:any` of Ruby's `:any?` symbol — real PostPilot
/// source, `delegate :any?, :empty?, :none?, to: :campaigns`, tripped it with
/// nothing about `:any?` being untyped in the way `any` is.
#[cfg(test)]
mod ruby_does_not_read_typescripts_pattern {
    use crate::catalog::Catalog;
    use crate::checks::{self, Ctx};
    use crate::policy::Policy;
    use crate::ratchet::Ratchet;
    use crate::scan;
    use std::path::Path;

    /// A scratch checkout on disk: this check reads `std::fs`, so there is no
    /// way to exercise it without real files at real paths.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock is before the epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("sf-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("scratch directory");
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const POLICY: &str = "version: 1\n\
        project:\n  name: scoping\n  languages: [ruby]\n\
        rules:\n  L1.NO_UNTYPED_ESCAPE_HATCH:\n    enabled: true\n";

    fn findings_for(root: &Path, source: &str) -> Vec<crate::finding::Finding> {
        std::fs::create_dir_all(root.join(".software-factory")).expect(".software-factory");
        std::fs::write(root.join(".software-factory/policy.yaml"), POLICY).expect("policy.yaml");
        std::fs::write(root.join("app.rb"), source).expect("app.rb");

        let catalog = Catalog::builtin().expect("builtin catalog");
        let policy = Policy::load(root).expect("policy loads");
        let files = scan::walk(root, &policy).expect("walk");
        let ratchet = Ratchet::default();
        let rule =
            catalog.get("L1.NO_UNTYPED_ESCAPE_HATCH").expect("the rule ships in the catalog");
        let ctx = Ctx {
            root,
            policy: &policy,
            catalog: &catalog,
            files: &files,
            ratchet: &ratchet,
            changed: None,
            base: None,
            today: crate::clock::today(),
            allow_commands: false,
        };
        checks::run_one(rule, &ctx).expect("check runs")
    }

    #[test]
    fn delegate_any_predicate_is_not_the_untyped_escape_hatch() {
        let scratch = Scratch::new("text-pattern-ruby-any");
        let findings = findings_for(
            scratch.0.as_path(),
            "class CampaignCollectionPresenter\n  \
             delegate :any?, :empty?, :none?, to: :campaigns\nend\n",
        );
        assert!(
            findings.is_empty(),
            "the TypeScript `any` pattern fired on a Ruby `:any?` symbol: {:?}",
            findings.iter().map(|f| f.actual.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn t_dot_untyped_still_fires_on_ruby() {
        let scratch = Scratch::new("text-pattern-ruby-untyped");
        let findings = findings_for(
            scratch.0.as_path(),
            "sig { params(event: T.untyped).returns(T.untyped) }\n\
             def handle(event)\n  event\nend\n",
        );
        assert!(
            !findings.is_empty(),
            "the rule's own escape hatch, T.untyped, must still fire on Ruby"
        );
    }
}
