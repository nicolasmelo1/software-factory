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
            grouped.entry(finding.rule.as_str()).or_default().push(finding);
        }
        let mut out = String::new();
        for (rule_id, findings) in &grouped {
            let rule = catalog.get(rule_id);
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
            if let Some(rule) = catalog.get(rule_id) {
                out.push_str(&format!("\n### {} — {}\n\n{}\n\n**Fix.** {}\n", rule.id, rule.title, rule.why, rule.fix));
            }
        }
        out
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
