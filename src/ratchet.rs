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

    /// Replace a rule's frozen set with exactly today's violations.
    pub fn seed(&mut self, rule: &str, keys: BTreeSet<String>, review_by: &str) {
        if keys.is_empty() {
            self.rules.remove(rule);
            return;
        }
        let entry = self.rules.entry(rule.to_string()).or_default();
        entry.allow = keys;
        entry.review_by = review_by.to_string();
    }
}
