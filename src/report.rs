//! Rendering. The terminal output is deliberately verbose about *why*: the
//! failure message is the only documentation an agent reliably reads.
//!
//! The trail is the dynamic half of that output. The `fix` renders identically
//! every run, so a looping model receives the same bytes and learns nothing;
//! once the escape log has attempts against a finding key, the finding also
//! renders what has already been tried, and past the threshold a worked
//! repair joins it. An empty log renders byte-for-byte as before.

use crate::catalog::Catalog;
use crate::finding::{Finding, Severity, EXIT_FINDINGS, EXIT_OK};
use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;

/// What the report can say about one stuck finding, read from the escape log
/// by the caller (the check loop that owns the root and the rule ids).
#[derive(Debug, Default, Serialize)]
pub struct Trail {
    /// Human-visible counts of failed edits, newest last, as rendered.
    pub attempts: Vec<String>,
    /// The worked repair, when the caller decided one is due and a clean one
    /// exists. `None` renders as the plain finding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escape: Option<String>,
}

impl Trail {
    pub fn has_content(&self) -> bool {
        !self.attempts.is_empty() || self.escape.is_some()
    }
}

#[derive(Serialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub frozen: usize,
    pub rules_run: usize,
    /// What the escape log says about each stuck finding key. Keys absent
    /// from the map have no trail, and absent is the ordinary case: a first
    /// run of `sf check` in a repository with a cold log renders identically
    /// to the version without this feature.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub trail: BTreeMap<String, Trail>,
}

impl Report {
    pub fn exit_code(&self) -> i32 {
        if self.findings.is_empty() {
            EXIT_OK
        } else {
            EXIT_FINDINGS
        }
    }

    pub fn json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn text(&self, catalog: &Catalog) -> String {
        if self.findings.is_empty() {
            return format!(
                "✓ {} rules, no findings{}\n",
                self.rules_run,
                if self.frozen > 0 {
                    format!(" ({} frozen by the ratchet)", self.frozen)
                } else {
                    String::new()
                }
            );
        }
        let mut grouped: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
        for finding in &self.findings {
            grouped
                .entry(finding.rule.as_str())
                .or_default()
                .push(finding);
        }
        let mut out = String::new();
        for (rule_id, findings) in &grouped {
            // An instance (`RULE@name`, documented in the README for a monorepo
            // that needs one rule twice) is not itself a catalog entry, so a
            // direct lookup misses and the report loses the title, the `why`
            // and the `fix` — the agent-facing documentation this rule exists
            // to hand over at the one moment somebody is trying to comply.
            let rule = catalog
                .get(rule_id)
                .or_else(|| catalog.get(crate::policy::base_rule_id(rule_id)));
            let title = rule.map(|r| r.title.as_str()).unwrap_or("(unknown rule)");
            let severity = findings[0].severity;
            out.push_str(&format!("\n{} {rule_id} — {title}\n", marker(severity)));
            if let Some(rule) = rule {
                out.push_str(&format!("  why  {}\n", wrap(&rule.why, "       ")));
                out.push_str(&format!("  fix  {}\n", wrap(&rule.fix, "       ")));
            }
            for finding in findings {
                out.push_str(&format!("    {} — {}\n", finding.location, finding.message));
                if let Some(expected) = &finding.expected {
                    out.push_str(&format!("       expected {expected}\n"));
                }
                if let Some(actual) = &finding.actual {
                    out.push_str(&format!("       actual   {actual}\n"));
                }
                out.push_str(&trail_lines(&self.trail, &finding.key, "       "));
            }
        }
        out.push_str(&format!(
            "\n{} findings across {} rules{}\n",
            self.findings.len(),
            grouped.len(),
            if self.frozen > 0 {
                format!(" ({} frozen by the ratchet)", self.frozen)
            } else {
                String::new()
            }
        ));
        out
    }

    pub fn markdown(&self, catalog: &Catalog) -> String {
        let mut out = String::from("# Software factory report\n\n");
        if self.findings.is_empty() {
            out.push_str(&format!(
                "No findings across {} enabled rules.\n",
                self.rules_run
            ));
            return out;
        }
        out.push_str("| Rule | Location | Finding |\n| --- | --- | --- |\n");
        for finding in &self.findings {
            out.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                finding.rule, finding.location, finding.message
            ));
        }
        let mut seen: Vec<&str> = self.findings.iter().map(|f| f.rule.as_str()).collect();
        seen.sort();
        seen.dedup();
        out.push_str("\n## Why these rules exist\n");
        for rule_id in seen {
            // Same instance fallback as the text report above.
            if let Some(rule) = catalog
                .get(rule_id)
                .or_else(|| catalog.get(crate::policy::base_rule_id(rule_id)))
            {
                out.push_str(&format!(
                    "\n### {} — {}\n\n{}\n\n**Fix.** {}\n",
                    rule.id, rule.title, rule.why, rule.fix
                ));
            }
        }
        out
    }
}

/// The attempt trail and the worked escape, under one finding. Escaped
/// entirely when the log has nothing to say about this key, which is what
/// keeps the ordinary finding's bytes identical to the version without it.
fn trail_lines(trail: &BTreeMap<String, Trail>, key: &str, indent: &str) -> String {
    let Some(entry) = trail.get(key) else {
        return String::new();
    };
    if !entry.has_content() {
        return String::new();
    }
    let mut out = String::new();
    for attempt in &entry.attempts {
        out.push_str(&format!("{indent}tried   {attempt}\n"));
    }
    if let Some(escape) = &entry.escape {
        for line in escape.lines() {
            out.push_str(&format!("{indent}escape  {line}\n"));
        }
    }
    out
}

fn marker(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "✗ critical",
        Severity::High => "✗ high",
        Severity::Medium => "! medium",
        Severity::Low => "· low",
    }
}

/// Re-wrap prose at 76 columns so a `why` stays readable in a CI log.
fn wrap(text: &str, indent: &str) -> String {
    let mut line = String::new();
    let mut out = String::new();
    for word in text.split_whitespace() {
        if line.len() + word.len() + 1 > 70 {
            out.push_str(&format!("{line}\n{indent}"));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    out.push_str(&line);
    out
}

/// `RULE@name` is documented and an instance is not itself a catalog entry.
/// Before the fallback in `text` and `markdown`, an instance's finding rendered
/// as `(unknown rule)` with no `why` and no `fix` — dropping the prose the rule
/// exists to hand over, at the moment somebody is trying to comply.
///
/// Nothing caught it: this repository had no instance of its own until
/// `L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE@release`.
#[cfg(test)]
mod an_instance_keeps_its_prose {
    use super::*;

    fn report_for(rule_id: &str) -> Report {
        Report {
            findings: vec![Finding::new(
                rule_id,
                Severity::Medium,
                "src/a.rs",
                "key",
                "a finding from an instance",
            )],
            frozen: 0,
            rules_run: 1,
            trail: BTreeMap::new(),
        }
    }

    #[test]
    fn the_text_report_resolves_an_instance_to_its_base_rule() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rendered = report_for("L1.COMPLEXITY_CEILING@legacy").text(&catalog);
        assert!(
            !rendered.contains("(unknown rule)"),
            "an instance lost its title: {rendered}"
        );
        assert!(rendered.contains("No function exceeds the cyclomatic ceiling"));
        assert!(
            rendered.contains("  why  "),
            "an instance lost its why: {rendered}"
        );
        assert!(
            rendered.contains("  fix  "),
            "an instance lost its fix: {rendered}"
        );
        // The instance id itself still has to be what the report names, or
        // there is no way to tell which of two instances fired.
        assert!(rendered.contains("L1.COMPLEXITY_CEILING@legacy"));
    }

    #[test]
    fn the_markdown_report_resolves_an_instance_too() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rendered = report_for("L1.COMPLEXITY_CEILING@legacy").markdown(&catalog);
        assert!(
            rendered.contains("No function exceeds the cyclomatic ceiling"),
            "the markdown report dropped the instance's prose: {rendered}"
        );
    }

    /// A genuinely unknown id must still say so. The fallback splits on `@`,
    /// so a bare id that is not in the catalog has to keep reporting as
    /// unknown rather than resolving to something.
    #[test]
    fn an_id_that_is_not_in_the_catalog_still_reads_as_unknown() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rendered = report_for("L9.NOT_A_RULE@instance").text(&catalog);
        assert!(rendered.contains("(unknown rule)"), "{rendered}");
    }
}

#[cfg(test)]
mod the_attempt_trail {
    use super::*;

    fn finding_with(key: &str) -> Finding {
        Finding::new(
            "L1.COMPLEXITY_CEILING",
            Severity::Medium,
            "src/a.rs:1",
            key,
            "`price` has 13 independent paths, ceiling is 12",
        )
    }

    /// The property the feature exists for: once attempts accumulate, the
    /// rendered finding changes instead of repeating byte for byte.
    #[test]
    fn a_rendered_finding_changes_once_attempts_accumulate() {
        let plain = Report {
            findings: vec![finding_with("src/a.rs:price")],
            frozen: 0,
            rules_run: 1,
            trail: BTreeMap::new(),
        };
        let mut trail = BTreeMap::new();
        trail.insert(
            "src/a.rs:price".to_string(),
            Trail {
                attempts: vec![
                    "attempt 1 — diff f0c1".to_string(),
                    "attempt 2 — diff b2e4".to_string(),
                ],
                escape: None,
            },
        );
        let repeated = Report {
            findings: vec![finding_with("src/a.rs:price")],
            frozen: 0,
            rules_run: 1,
            trail,
        };
        let first = plain.text(&Catalog::builtin().expect("the catalog loads"));
        let second = plain.text(&Catalog::builtin().expect("the catalog loads"));
        assert_eq!(
            first, second,
            "an empty log must render byte-identical runs"
        );
        let with = repeated.text(&Catalog::builtin().expect("the catalog loads"));
        assert_ne!(first, with, "the trail changes what the finding renders");
        assert!(with.contains("tried"), "the attempts render: {with}");
        assert!(with.contains("attempt 1"), "each attempt is named: {with}");
    }

    /// The ladder: at two attempts the model sees only its own trail; past
    /// the threshold a worked escape joins. Byte counts differ because the
    /// content differs, which is the gradient.
    #[test]
    fn the_escape_joins_only_past_the_attempt_ceiling() {
        let mut trail = BTreeMap::new();
        trail.insert(
            "src/a.rs:price".to_string(),
            Trail {
                attempts: vec!["attempt 1".to_string()],
                escape: Some("worked repair: extract the loop".to_string()),
            },
        );
        let report = Report {
            findings: vec![finding_with("src/a.rs:price")],
            frozen: 0,
            rules_run: 1,
            trail,
        };
        let rendered = report.text(&Catalog::builtin().expect("the catalog loads"));
        assert!(
            rendered.contains("escape"),
            "the worked repair renders: {rendered}"
        );
        assert!(rendered.contains("worked repair"), "{rendered}");
    }

    #[test]
    fn a_trail_for_another_key_leaves_the_finding_alone() {
        let mut trail = BTreeMap::new();
        trail.insert(
            "src/other.rs:parse".to_string(),
            Trail {
                attempts: vec!["attempt 1".to_string()],
                escape: Some("worked repair".to_string()),
            },
        );
        let report = Report {
            findings: vec![finding_with("src/a.rs:price")],
            frozen: 0,
            rules_run: 1,
            trail,
        };
        let rendered = report.text(&Catalog::builtin().expect("the catalog loads"));
        assert!(
            !rendered.contains("tried"),
            "no cross-key bleed: {rendered}"
        );
    }

    #[test]
    fn the_json_report_carries_the_trail_as_data() {
        let mut trail = BTreeMap::new();
        trail.insert(
            "src/a.rs:price".to_string(),
            Trail {
                attempts: vec!["attempt 1".to_string()],
                escape: None,
            },
        );
        let report = Report {
            findings: vec![finding_with("src/a.rs:price")],
            frozen: 0,
            rules_run: 1,
            trail,
        };
        let json = report.json().expect("the report serialises");
        assert!(json.contains("\"trail\""), "{json}");
        assert!(json.contains("\"attempts\""), "{json}");
        // And a clean run carries none of it.
        let plain = Report {
            findings: vec![],
            frozen: 0,
            rules_run: 1,
            trail: BTreeMap::new(),
        };
        assert!(
            !plain.json().expect("the report serialises").contains("\"trail\""),
            "absent when empty"
        );
    }
}
