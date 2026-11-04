//! L5 — a property a plan cites is proven able to fail.
//!
//! `(proof: property:PATH#FALSIFIER)` names the mutation the property must
//! reject. The rule's `run` command runs with `SF_PROPERTY` set to PATH, once
//! as the code is and once with `SF_FALSIFIER` naming the mutation to plant:
//! the first must pass and the second fail. The why is the rule's prose.

use super::Ctx;
use super::cadence::parse_criteria;
use crate::catalog::Rule;
use crate::finding::{Finding, Severity};
use crate::policy::Options;
use crate::scan;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// Set for every run to the PATH the citation names.
pub const PROPERTY_VAR: &str = "SF_PROPERTY";
/// Set to the falsifier's name for the planted run, and unset for the other.
pub const FALSIFIER_VAR: &str = "SF_FALSIFIER";

/// The two halves of a `property:` marker's value, `PATH#FALSIFIER`, when
/// both are there. A falsifier is one word: it is a name the property file
/// has to carry, not a sentence about one.
pub fn split(value: &str) -> Option<(&str, &str)> {
    let (path, falsifier) = value.trim().rsplit_once('#')?;
    let (path, falsifier) = (path.trim(), falsifier.trim());
    let word = !falsifier.is_empty() && !falsifier.contains(char::is_whitespace);
    (!path.is_empty() && word).then_some((path, falsifier))
}

/// Whether `text` carries `name` as a whole token: not inside a longer
/// identifier on either side.
///
/// A regex word boundary cannot close after a name ending in punctuation
/// (`drops_last_line!` before a quote), and `split` accepts any name without
/// whitespace, so the edges are checked by hand instead: the characters
/// beside an occurrence must not continue an identifier.
fn names_token(text: &str, name: &str) -> bool {
    let continues = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    text.match_indices(name).any(|(at, _)| {
        !continues(text[..at].chars().next_back())
            && !continues(text[at + name.len()..].chars().next())
    })
}

/// One property a plan cites, at the first criterion that cites it.
struct Citation {
    at: String,
    path: String,
    falsifier: String,
}

/// `L5.EVERY_PROPERTY_HAS_A_FALSIFIER`.
pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let mut cited: BTreeMap<(String, String), Citation> = BTreeMap::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Ok(body) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        for criterion in parse_criteria(&body)? {
            if criterion.kind.as_deref() != Some("property") {
                continue;
            }
            let at = format!("{}:{}", file.rel, criterion.line);
            match named(ctx.root, &criterion.value) {
                Err((key, message, expected)) => findings.push(
                    Finding::new(&rule.id, rule.severity, &at, format!("{key}:{at}"), message)
                        .expected(expected)
                        .actual(format!("property:{}", criterion.value)),
                ),
                Ok((path, falsifier)) => {
                    cited
                        .entry((path.clone(), falsifier.clone()))
                        .or_insert(Citation {
                            at,
                            path,
                            falsifier,
                        });
                }
            }
        }
    }
    findings.extend(exercise(rule, opts, ctx, cited.into_values().collect())?);
    Ok(findings)
}

/// A refusal before anything runs: its key, message and expected form.
type Refusal = (&'static str, String, String);

/// The path and falsifier a marker names, once both are shown to exist.
fn named(root: &Path, value: &str) -> Result<(String, String), Refusal> {
    let Some((path, falsifier)) = split(value) else {
        return Err((
            "no-falsifier",
            "this property names no falsifier: nothing shows it can fail, so it proves nothing"
                .to_string(),
            "property:PATH#FALSIFIER, the implementation mutation the property must reject"
                .to_string(),
        ));
    };
    let Ok(text) = std::fs::read_to_string(root.join(path)) else {
        return Err((
            "missing-property",
            format!("this property cites `{path}`, which does not exist"),
            format!("the property at `{path}`"),
        ));
    };
    if !names_token(&text, falsifier) {
        return Err((
            "missing-falsifier",
            format!(
                "falsifier `{falsifier}` is named by the plan and never written in `{path}`: there is no mutation for the property to reject"
            ),
            format!("`{path}` planting `{falsifier}` when {FALSIFIER_VAR} names it"),
        ));
    }
    Ok((path.to_string(), falsifier.to_string()))
}

/// Run each cited property both ways: as the code is, and with its
/// falsifier planted. The first run is shared by every falsifier of a path.
fn exercise(rule: &Rule, opts: &Options, ctx: &Ctx, cited: Vec<Citation>) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    if cited.is_empty() {
        return Ok(findings);
    }
    let Some(command) = opts.run.as_deref() else {
        return Ok(cited
            .iter()
            .map(|c| {
                finding(rule, rule.severity, c, "unrunnable", "nothing is configured to run this property, so nothing shows it passes or that it can fail")
                    .expected(format!("`run:` on {}: the command that runs the property at ${PROPERTY_VAR}", rule.id))
            })
            .collect());
    };
    if !ctx.allow_commands {
        return Ok(cited
            .iter()
            .map(|c| {
                finding(
                    rule,
                    Severity::Medium,
                    c,
                    "not-run",
                    "this property runs a command, and commands are not enabled",
                )
                .expected("sf check --allow-commands, or SF_ALLOW_COMMANDS=1")
                .actual(command.to_string())
            })
            .collect());
    }
    let mut unmutated: BTreeMap<String, Ran> = BTreeMap::new();
    for citation in &cited {
        if !unmutated.contains_key(&citation.path) {
            let ran = run_once(ctx.root, command, &citation.path, None)?;
            if !ran.passed {
                findings.push(
                    finding(rule, rule.severity, citation, "fails-unmutated", &format!("the property at `{}` fails on the code as it is, so a failure with a falsifier planted proves nothing", citation.path))
                        .expected("exit 0 with no falsifier planted")
                        .actual(ran.detail.clone()),
                );
            }
            unmutated.insert(citation.path.clone(), ran);
        }
        if !unmutated[&citation.path].passed {
            continue;
        }
        let planted = run_once(ctx.root, command, &citation.path, Some(&citation.falsifier))?;
        if planted.passed {
            findings.push(
                finding(rule, rule.severity, citation, "unfalsified", &format!("the property at `{}` passes with `{}` planted: it cannot fail, so it cannot be cited", citation.path, citation.falsifier))
                    .expected(format!("a non-zero exit with {FALSIFIER_VAR}={}", citation.falsifier))
                    .actual("exit 0"),
            );
        }
    }
    Ok(findings)
}

fn finding(rule: &Rule, severity: Severity, c: &Citation, key: &str, message: &str) -> Finding {
    Finding::new(
        &rule.id,
        severity,
        &c.at,
        format!("{key}:{}#{}", c.path, c.falsifier),
        message.to_string(),
    )
}

/// What one run of the command said.
struct Ran {
    passed: bool,
    detail: String,
}

fn run_once(root: &Path, command: &str, path: &str, falsifier: Option<&str>) -> Result<Ran> {
    let mut run = Command::new("sh");
    run.arg("-c")
        .arg(command)
        .current_dir(root)
        .env(PROPERTY_VAR, path);
    match falsifier {
        Some(name) => run.env(FALSIFIER_VAR, name),
        None => run.env_remove(FALSIFIER_VAR),
    };
    let output = run.output()?;
    let mut detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        detail = String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    let lines: Vec<&str> = detail.lines().collect();
    Ok(Ran {
        passed: output.status.success(),
        detail: lines[lines.len().saturating_sub(20)..].join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::{names_token, split};
    use crate::catalog::Catalog;
    use crate::checks::{Ctx, run_one};
    use crate::policy::Policy;
    use crate::ratchet::Ratchet;

    /// A property honest about its falsifier: passes as it is, fails when
    /// `zero_read_as_unset` is planted.
    const HONEST: &str = "# total() keeps an explicit zero.\ncase \"${SF_FALSIFIER:-}\" in\n  zero_read_as_unset) exit 1 ;;\nesac\nexit 0\n";
    /// Names the falsifier and never plants it: the generator never reaches
    /// the case, and it passes either way.
    const VACUOUS: &str =
        "# total() keeps an explicit zero; zero_read_as_unset is never planted.\nexit 0\n";
    /// Fails on the code as it is, so it "rejects" every falsifier.
    const BROKEN: &str = "# total() keeps an explicit zero; zero_read_as_unset.\nexit 1\n";

    fn plan(marker: &str) -> String {
        format!("# Totals\n\n- [ ] An explicit zero survives. (proof: {marker})\n")
    }

    /// The rule's findings over one plan citing `marker`, as their keys.
    fn keys(marker: &str, property: &str, run: Option<&str>, allow_commands: bool) -> Vec<String> {
        let plan = plan(marker);
        let root = crate::checks::runtime_pin::scratch(
            "property",
            &[
                ("plans/totals.md", plan.as_str()),
                ("props/total.sh", property),
            ],
        );
        let options = run
            .map(|run| format!("    options:\n      run: {run:?}\n"))
            .unwrap_or_default();
        let policy: Policy = serde_yaml::from_str(&format!(
            "version: 1\nproject:\n  name: p\n  languages: [rust]\nrules:\n  L5.EVERY_PROPERTY_HAS_A_FALSIFIER:\n    enabled: true\n{options}"
        ))
        .expect("the policy parses");
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rule = catalog
            .get("L5.EVERY_PROPERTY_HAS_A_FALSIFIER")
            .expect("the rule ships");
        let files = crate::scan::walk(&root, &policy).expect("the scratch walks");
        let ratchet = Ratchet::default();
        let ctx = Ctx {
            root: &root,
            policy: &policy,
            catalog: &catalog,
            files: &files,
            ratchet: &ratchet,
            changed: None,
            base: None,
            today: "2026-01-01".to_string(),
            allow_commands,
            overlay: None,
        };
        let found = run_one(rule, &ctx).expect("the rule runs");
        let _ = std::fs::remove_dir_all(&root);
        found
            .into_iter()
            .map(|f| f.key.split(':').next().unwrap_or_default().to_string())
            .collect()
    }

    const RUN: Option<&str> = Some("sh \"$SF_PROPERTY\"");

    #[test]
    fn a_marker_splits_into_a_path_and_one_word() {
        assert_eq!(
            split("src/merge.rs#zero_read_as_unset"),
            Some(("src/merge.rs", "zero_read_as_unset"))
        );
        for refused in ["src/merge.rs", "src/merge.rs#", "#zero", "a.rs#two words"] {
            assert_eq!(split(refused), None, "{refused}");
        }
    }

    /// A name is matched as a whole token, punctuation included, and never
    /// inside a longer identifier.
    #[test]
    fn a_falsifier_is_found_as_a_whole_token() {
        assert!(names_token(
            "case \"$SF_FALSIFIER\" in\n  drops_last_line!) exit 1 ;;",
            "drops_last_line!"
        ));
        assert!(names_token(
            "  zero_read_as_unset) exit 1 ;;",
            "zero_read_as_unset"
        ));
        assert!(!names_token("  zero_read_as_unset) exit 1 ;;", "zero"));
        assert!(!names_token("  unset_zero) exit 1 ;;", "zero"));
        assert!(!names_token("nothing here", "zero"));
    }

    /// A2: the named falsifier has to exist. Absent from the property file,
    /// it is a finding before anything runs, with or without commands; with
    /// it written, the same citation passes.
    #[test]
    fn a_falsifier_the_property_never_names_is_a_finding_and_a_named_one_passes() {
        for allow in [false, true] {
            assert_eq!(
                keys("property:props/total.sh#never_written", HONEST, RUN, allow),
                vec!["missing-falsifier"]
            );
        }
        assert_eq!(
            keys("property:props/total.sh", HONEST, RUN, true),
            vec!["no-falsifier"],
            "a property with no falsifier is a finding, not a pass"
        );
        assert_eq!(
            keys(
                "property:props/elsewhere.sh#zero_read_as_unset",
                HONEST,
                RUN,
                true
            ),
            vec!["missing-property"]
        );
        assert!(
            keys(
                "property:props/total.sh#zero_read_as_unset",
                HONEST,
                RUN,
                true
            )
            .is_empty(),
            "named, planted and rejected: the citation stands"
        );
    }

    /// A3: both directions. A property that passes with its falsifier planted
    /// cannot fail, and one that fails without it rejects everything.
    #[test]
    fn a_falsifier_the_property_does_not_reject_is_a_finding() {
        assert_eq!(
            keys(
                "property:props/total.sh#zero_read_as_unset",
                VACUOUS,
                RUN,
                true
            ),
            vec!["unfalsified"]
        );
        assert_eq!(
            keys(
                "property:props/total.sh#zero_read_as_unset",
                BROKEN,
                RUN,
                true
            ),
            vec!["fails-unmutated"],
            "a property that always fails rejects every falsifier, and proves nothing by it"
        );
    }

    /// Nothing is run without `--allow-commands`, and nothing that did not
    /// run counts as passed; with no `run` configured there is nothing to run.
    #[test]
    fn an_unrun_property_is_never_a_pass() {
        assert_eq!(
            keys(
                "property:props/total.sh#zero_read_as_unset",
                HONEST,
                RUN,
                false
            ),
            vec!["not-run"]
        );
        assert_eq!(
            keys(
                "property:props/total.sh#zero_read_as_unset",
                HONEST,
                None,
                true
            ),
            vec!["unrunnable"]
        );
    }
}
