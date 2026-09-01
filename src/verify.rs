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
            detail: gated_off(&policy, fixture_root, rule_id)?
                .unwrap_or_else(|| "the mutation did not trip the rule".to_string()),
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

/// Why nothing fired, when the reason is that every enabled instance of the
/// rule was gated off by a `when` the fixture does not satisfy.
///
/// A fixture for a conditional rule has to carry the manifest that satisfies
/// its condition, or the run proves nothing about it. Saying "the mutation did
/// not trip the rule" there sends whoever reads it to the query, which is not
/// where the problem is. `None` means the gate is not the explanation: either
/// the rule has no condition, or one of its instances ran and found nothing.
fn gated_off(policy: &Policy, root: &Path, rule_id: &str) -> Result<Option<String>> {
    let instances: Vec<String> = policy
        .instances()
        .into_iter()
        .filter(|(_, base)| base == rule_id)
        .map(|(instance, _)| instance)
        .collect();
    let mut reasons = Vec::new();
    for instance in &instances {
        let activation = policy.activation_of(root, instance)?;
        match activation.stale_reason() {
            Some(reason) => reasons.push(format!("{instance} — {reason}")),
            None => return Ok(None),
        }
    }
    if reasons.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "nothing ran: the fixture does not satisfy the condition on {}",
        reasons.join("; ")
    )))
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

#[cfg(test)]
mod conditional_fixtures {
    use super::*;
    use crate::policy::FIXTURES_DIR;

    fn fixture_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES_DIR).join("L5.NO_INERT_RULE")
    }

    /// The `L5.NO_INERT_RULE` fixture is the one carrying `when` conditions,
    /// and it still has to trip the rule it is a fixture for. A conditional
    /// instance that stops a fixture firing would make `sf verify` green on a
    /// rule nothing proved.
    #[test]
    fn a_fixture_holding_conditional_instances_still_trips_its_own_rule() {
        let catalog = Catalog::builtin().expect("the built-in catalog loads");
        let outcome = run_fixture(&fixture_root(), "L5.NO_INERT_RULE", &catalog, false)
            .expect("the fixture runs");
        assert!(outcome.fired, "{}", outcome.detail);
    }

    /// And the failure this makes legible: a fixture whose manifest does not
    /// satisfy the rule's own condition. The fixture pins `tailwindcss
    /// ^4.0.2`, so a policy whose only instances are written for `^3` proves
    /// nothing, and has to say that rather than blaming the mutation.
    #[test]
    fn a_fixture_that_does_not_satisfy_the_condition_says_so_instead_of_blaming_the_mutation() {
        let policy: Policy = serde_yaml::from_str(
            "version: 1\nproject:\n  name: gated\n  languages: [python]\nrules:\n  L1.NO_BLANKET_SUPPRESSION@tailwind3:\n    enabled: true\n    when:\n      dependency: tailwindcss\n      manifest: package.json\n      version: \"^3\"\n",
        )
        .expect("the policy parses");
        let detail = gated_off(&policy, &fixture_root(), "L1.NO_BLANKET_SUPPRESSION")
            .expect("the condition is decidable")
            .expect("every instance is gated off");
        assert!(detail.contains("^3"), "names the range the fixture was written for: {detail}");
        assert!(detail.contains("^4.0.2"), "names what the fixture actually pins: {detail}");

        // A rule with no condition at all is not explained by this: the
        // fixture is simply not tripping it.
        let unconditional: Policy = serde_yaml::from_str(
            "version: 1\nproject:\n  name: plain\n  languages: [python]\nrules:\n  L1.NO_BLANKET_SUPPRESSION:\n    enabled: true\n",
        )
        .expect("the policy parses");
        assert_eq!(
            gated_off(&unconditional, &fixture_root(), "L1.NO_BLANKET_SUPPRESSION")
                .expect("the condition is decidable"),
            None
        );
    }
}
