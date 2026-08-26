//! The ratchet: how a rule gets adopted by a repository that already violates it.
//!
//! Seeding freezes today's violations by key and lets everything new fail. The
//! entries are not permission — they are a dated debt list, and
//! `L2.NO_PERMANENT_EXCEPTION` fails the build when the date passes.

use crate::finding::Finding;
use crate::policy::RATCHET_PATH;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RuleRatchet {
    /// ISO date. Past this day the frozen entries stop being accepted.
    pub review_by: String,
    #[serde(default)]
    pub note: Option<String>,
    /// Stable finding keys, one per grandfathered violation.
    #[serde(default)]
    pub allow: BTreeSet<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ratchet {
    pub version: u32,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleRatchet>,
}

impl Default for Ratchet {
    fn default() -> Self {
        Ratchet { version: 1, rules: BTreeMap::new() }
    }
}

impl Ratchet {
    pub fn load(root: &Path) -> Result<Ratchet> {
        let path = root.join(RATCHET_PATH);
        if !path.exists() {
            return Ok(Ratchet::default());
        }
        let body = std::fs::read_to_string(&path)?;
        serde_yaml::from_str(&body).with_context(|| format!("{} is malformed", path.display()))
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let path = root.join(RATCHET_PATH);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let header = "# Frozen violations, one key per line, per rule.\n\
                      # These are debt with a due date, not permission. Adding a key by hand\n\
                      # to silence a new violation is the one move this file exists to make\n\
                      # visible in review.\n";
        std::fs::write(&path, format!("{header}{}", serde_yaml::to_string(self)?))?;
        Ok(())
    }

    pub fn allows(&self, rule: &str, key: &str) -> bool {
        self.rules.get(rule).is_some_and(|r| r.allow.contains(key))
    }

    /// Mark findings the ratchet covers. Returns only the ones that still fail.
    pub fn apply(&self, findings: Vec<Finding>) -> (Vec<Finding>, usize) {
        let mut live = Vec::new();
        let mut frozen = 0;
        for finding in findings {
            if self.allows(&finding.rule, &finding.key) {
                frozen += 1;
            } else {
                live.push(finding);
            }
        }
        (live, frozen)
    }

    /// Replace a rule's frozen set with exactly today's violations, carrying
    /// `previous`'s date and note forward.
    ///
    /// The date only ever moves closer. A re-seed is not a way to buy another
    /// six months on debt already counted: `sf ratchet` is part of the
    /// prescribed order after any guardrail change, so a run that stamped
    /// today + N months on untouched entries would push every date out on
    /// every unrelated change — which `L2.POLICY_ONLY_TIGHTENS` correctly
    /// rejects, leaving the repository unable to follow its own instructions.
    /// Renewing a date that has genuinely expired is a human's edit, with the
    /// reasoning in the pull request.
    pub fn seed(
        &mut self,
        rule: &str,
        keys: BTreeSet<String>,
        review_by: &str,
        previous: Option<&RuleRatchet>,
    ) {
        if keys.is_empty() {
            self.rules.remove(rule);
            return;
        }
        let entry = self.rules.entry(rule.to_string()).or_default();
        entry.allow = keys;
        entry.review_by = review_by.to_string();
        let Some(previous) = previous else { return };
        // ISO dates order lexicographically, which is the same comparison
        // `L2.NO_PERMANENT_EXCEPTION` makes against today.
        if !previous.review_by.is_empty() && previous.review_by < entry.review_by {
            entry.review_by = previous.review_by.clone();
        }
        entry.note = previous.note.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frozen(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    fn seeded(review_by: &str, note: Option<&str>, keys: &[&str]) -> RuleRatchet {
        RuleRatchet {
            review_by: review_by.to_string(),
            note: note.map(|n| n.to_string()),
            allow: frozen(keys),
        }
    }

    #[test]
    fn a_re_seed_never_pushes_a_review_date_out() {
        let previous = seeded("2027-02-18", Some("measured, not guessed"), &["rust"]);
        let mut ratchet = Ratchet::default();
        ratchet.seed("L6.PERF", frozen(&["rust"]), "2027-02-25", Some(&previous));
        let entry = &ratchet.rules["L6.PERF"];
        assert_eq!(entry.review_by, "2027-02-18");
        assert_eq!(entry.note.as_deref(), Some("measured, not guessed"));
    }

    #[test]
    fn a_new_violation_does_not_reset_the_clock_on_the_old_debt() {
        let previous = seeded("2027-02-18", None, &["rust"]);
        let mut ratchet = Ratchet::default();
        ratchet.seed("L6.PERF", frozen(&["rust", "python"]), "2027-08-25", Some(&previous));
        assert_eq!(ratchet.rules["L6.PERF"].review_by, "2027-02-18");
    }

    #[test]
    fn a_rule_the_ratchet_has_never_seen_takes_the_new_date() {
        let mut ratchet = Ratchet::default();
        ratchet.seed("L6.PERF", frozen(&["rust"]), "2027-08-25", None);
        assert_eq!(ratchet.rules["L6.PERF"].review_by, "2027-08-25");
    }

    #[test]
    fn a_rule_with_nothing_left_to_freeze_leaves_the_file() {
        let previous = seeded("2027-02-18", None, &["rust"]);
        let mut ratchet = Ratchet::default();
        ratchet.seed("L6.PERF", BTreeSet::new(), "2027-08-25", Some(&previous));
        assert!(ratchet.rules.is_empty());
    }
}
