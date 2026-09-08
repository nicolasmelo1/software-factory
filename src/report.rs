//! Rendering. The terminal output is deliberately verbose about *why*: the
//! failure message is the only documentation an agent reliably reads.

use crate::catalog::Catalog;
use crate::finding::{EXIT_FINDINGS, EXIT_OK, Finding, Severity};
use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub frozen: usize,
    pub rules_run: usize,
    /// The policy that governed this run when it governed from outside:
    /// where it lives and which exact bytes it carried. Absent for a vendored
    /// run, where the policy is part of the repository and needs no
    /// introduction. A green overlay run must not be quotable later as a
    /// governed one, so every format names this — and that the run carried no
    /// ratchet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay: Option<crate::checks::Overlay>,
}

impl Report {
    pub fn exit_code(&self) -> i32 {
        if self.findings.is_empty() { EXIT_OK } else { EXIT_FINDINGS }
    }

    pub fn json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn text(&self, catalog: &Catalog) -> String {
        if self.findings.is_empty() {
            let mut green = format!(
                "✓ {} rules, no findings{}\n",
                self.rules_run,
                if self.frozen > 0 {
                    format!(" ({} frozen by the ratchet)", self.frozen)
                } else {
                    String::new()
                }
            );
            if let Some(overlay) = &self.overlay {
                green.push_str(&format!("  {}\n", overlay.disclaimer()));
            }
            return green;
        }
        let mut out = String::new();
        if let Some(overlay) = &self.overlay {
            out.push_str(&format!("  {}\n", overlay.disclaimer()));
        }
        let grouped = group_findings(&self.findings);
        let rules = grouped.len();
        for (rule_id, findings) in grouped {
            render_rule_group(catalog, &mut out, rule_id, &findings);
        }
        out.push_str(&format!(
            "\n{} findings across {} rules{}\n",
            self.findings.len(),
            rules,
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
        if let Some(overlay) = &self.overlay {
            out.push_str(&format!("> {}\n\n", overlay.disclaimer()));
        }
        if self.findings.is_empty() {
            out.push_str(&format!("No findings across {} enabled rules.\n", self.rules_run));
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
                out.push_str(&format!("\n### {} — {}\n\n{}\n\n**Fix.** {}\n", rule.id, rule.title, rule.why, rule.fix));
            }
        }
        out
    }
}

fn group_findings<'a>(findings: &'a [Finding]) -> BTreeMap<&'a str, Vec<&'a Finding>> {
    let mut grouped: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for finding in findings {
        grouped.entry(finding.rule.as_str()).or_default().push(finding);
    }
    grouped
}

/// One grouped rule's block: its prose, then every finding under it. The
/// instance fallback (`RULE@name` is documented and is not itself a catalog
/// entry) keeps an instance's `why` and `fix` in the report — the
/// agent-facing documentation this exists to hand over at the one moment
/// somebody is trying to comply.
fn render_rule_group<'a>(
    catalog: &Catalog,
    out: &mut String,
    rule_id: &str,
    findings: &[&'a Finding],
) {
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
    }
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
            overlay: None,
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
        assert!(rendered.contains("  why  "), "an instance lost its why: {rendered}");
        assert!(rendered.contains("  fix  "), "an instance lost its fix: {rendered}");
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
mod overlay_provenance {
    use super::Report;
    use crate::checks::Overlay;
    use crate::catalog::Catalog;

    fn report_with_overlay() -> Report {
        Report {
            findings: vec![],
            frozen: 0,
            rules_run: 3,
            overlay: Some(Overlay {
                path: "../factory-policy/.software-factory".to_string(),
                digest: "3857f5559a3e".to_string(),
            }),
        }
    }

    /// A green overlay run is the dangerous one: without the line, it reads
    /// exactly like a governed repository passing its own gate.
    #[test]
    fn every_format_names_the_overlay_on_a_green_run() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let report = report_with_overlay();
        let text = report.text(&catalog);
        assert!(text.contains("policy overlay: ../factory-policy/.software-factory"), "{text}");
        assert!(text.contains("no ratchet"), "{text}");
        assert!(text.contains("3857f5559a3e"), "{text}");
        let markdown = report.markdown(&catalog);
        assert!(markdown.contains("policy overlay:"), "{markdown}");
        assert!(markdown.contains("no ratchet"), "{markdown}");
    }

    #[test]
    fn a_json_report_carries_the_provenance_as_data() {
        let report = report_with_overlay();
        let json = report.json().expect("the report serialises");
        assert!(json.contains("\"overlay\""), "{json}");
        assert!(json.contains("\"digest\""), "{json}");
    }

    #[test]
    fn a_vendored_report_carries_no_disclaimer() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let plain = Report { findings: Vec::new(), frozen: 0, rules_run: 1, overlay: None };
        assert!(!plain.text(&catalog).contains("overlay"), "{}", plain.text(&catalog));
    }
}
