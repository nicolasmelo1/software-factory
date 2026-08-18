//! L5 — prove the checks fire.
//!
//! Every enabled rule is run against a mutation fixture that violates it. A
//! rule that reports nothing there is broken, and every green build it has
//! ever produced meant nothing.

use crate::catalog::Catalog;
use crate::checks::{self, Ctx};
use crate::clock;
use crate::policy::{FIXTURES_DIR, Policy};
use crate::ratchet::Ratchet;
use crate::scan;
use anyhow::Result;
use std::path::Path;

pub struct Outcome {
    pub rule: String,
    pub fired: bool,
    pub detail: String,
}

pub fn run(root: &Path, policy: &Policy, catalog: &Catalog, only: Option<&str>) -> Result<Vec<Outcome>> {
    let mut outcomes = Vec::new();
    for id in catalog.rules.keys() {
        if policy.enabled(id).is_none() {
            continue;
        }
        if only.is_some_and(|o| o != id) {
            continue;
        }
        let fixture_root = root.join(FIXTURES_DIR).join(id);
        if !fixture_root.is_dir() {
            outcomes.push(Outcome {
                rule: id.clone(),
                fired: false,
                detail: format!("no fixture at {FIXTURES_DIR}/{id}/"),
            });
            continue;
        }
        outcomes.push(run_fixture(&fixture_root, id, catalog)?);
    }
    Ok(outcomes)
}

fn run_fixture(fixture_root: &Path, rule_id: &str, catalog: &Catalog) -> Result<Outcome> {
    let policy = match Policy::load(fixture_root) {
        Ok(p) => p,
        Err(e) => {
            return Ok(Outcome {
                rule: rule_id.to_string(),
                fired: false,
                detail: format!("fixture policy could not be loaded: {e}"),
            });
        }
    };
    let mut fixture_catalog = Catalog::builtin()?;
    fixture_catalog.extend_from_dir(&fixture_root.join(crate::policy::RULES_DIR))?;
    let _ = catalog;
    let files = scan::walk(fixture_root, &policy)?;
    // The ratchet is deliberately ignored here except where the fixture is
    // *about* the ratchet: a frozen entry must never hide a mutation.
    let ratchet = if rule_id == "L2.NO_PERMANENT_EXCEPTION" {
        Ratchet::load(fixture_root)?
    } else {
        Ratchet::default()
    };
    let ctx = Ctx {
        root: fixture_root,
        policy: &policy,
        catalog: &fixture_catalog,
        files: &files,
        ratchet: &ratchet,
        changed: None,
        today: clock::today(),
    };
    let findings = checks::run_all(&ctx)?;
    let hits: Vec<_> = findings.iter().filter(|f| f.rule == rule_id).collect();
    Ok(Outcome {
        rule: rule_id.to_string(),
        fired: !hits.is_empty(),
        detail: if hits.is_empty() {
            "the mutation did not trip the rule".to_string()
        } else {
            format!("{} finding(s): {}", hits.len(), hits[0].message)
        },
    })
}
