//! L5 — prove the checks fire.
//!
//! Every enabled rule is run against a mutation fixture that violates it. A
//! rule that reports nothing there is broken, and every green build it has
//! ever produced meant nothing.

use crate::catalog::{Catalog, CheckKind};
use crate::checks::{self, Ctx};
use crate::clock;
use crate::policy::{FIXTURES_DIR, Policy};
use crate::lang::Lang;
use crate::ratchet::Ratchet;
use crate::scan;
use anyhow::Result;
use std::path::Path;

pub struct Outcome {
    pub rule: String,
    pub fired: bool,
    pub detail: String,
}

pub fn run(
    root: &Path,
    policy: &Policy,
    catalog: &Catalog,
    only: Option<&str>,
    allow_commands: bool,
) -> Result<Vec<Outcome>> {
    let mut outcomes = Vec::new();
    for id in catalog.rules.keys() {
        if !policy.any_instance_enabled(id) {
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
        outcomes.push(run_fixture(&fixture_root, id, catalog, allow_commands)?);
    }
    Ok(outcomes)
}

fn run_fixture(
    fixture_root: &Path,
    rule_id: &str,
    catalog: &Catalog,
    allow_commands: bool,
) -> Result<Outcome> {
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
        base: None,
        today: clock::today(),
        allow_commands,
    };
    let findings = checks::run_all(&ctx)?;
    let hits: Vec<_> = findings.iter().filter(|f| f.rule == rule_id).collect();
    if hits.is_empty() {
        return Ok(Outcome {
            rule: rule_id.to_string(),
            fired: false,
            detail: "the mutation did not trip the rule".to_string(),
        });
    }
    // A rule that carries a query per language passes here if *any* of them
    // fires, which would let three broken queries hide behind one working one.
    // Every language the rule claims has to be shown tripping it.
    if let Some(missing) = untested_languages(&fixture_catalog, rule_id, &hits) {
        return Ok(Outcome {
            rule: rule_id.to_string(),
            fired: false,
            detail: format!(
                "fires in some languages but the fixture never trips it in: {missing}"
            ),
        });
    }
    Ok(Outcome {
        rule: rule_id.to_string(),
        fired: true,
        detail: format!("{} finding(s): {}", hits.len(), hits[0].message),
    })
}

/// Languages a rule declares a query for but whose fixture files never fired.
fn untested_languages(
    catalog: &Catalog,
    rule_id: &str,
    hits: &[&crate::finding::Finding],
) -> Option<String> {
    let declared: Vec<String> = match catalog.get(rule_id).map(|r| &r.check) {
        Some(CheckKind::Shape { languages }) => languages.keys().cloned().collect(),
        Some(CheckKind::Nested { languages }) => languages.keys().cloned().collect(),
        _ => return None,
    };
    let fired: Vec<&str> = hits
        .iter()
        .filter_map(|f| {
            let path = f.location.split(':').next().unwrap_or("");
            Lang::from_path(Path::new(path)).map(|l| l.name())
        })
        .collect();
    let missing: Vec<String> =
        declared.into_iter().filter(|name| !fired.contains(&name.as_str())).collect();
    if missing.is_empty() {
        return None;
    }
    Some(missing.join(", "))
}
