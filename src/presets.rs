//! Named rule sets a policy can start from.
//!
//! A preset is the answer to "which rules?" for a whole class of
//! repositories — the part that takes a day of production to learn, written
//! once where every consumer inherits it. A policy names one with
//! `extends: <name>`; the loader merges the preset's rule settings under the
//! policy's own, one level, the policy's keys winning. Nothing else about the
//! preset travels: `project`, `gates` and `docs` stay the consumer's.
//!
//! The table is compiled in beside the catalog for the same reason the
//! catalog is: a scaffold written by another tool (`amy workflow new` writes
//! `extends: amy/workflow` into a policy) must resolve on a machine that has
//! never cloned this repository, and a preset is a promise about rule ids,
//! which are a public contract.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

/// The presets this binary carries. A policy naming one that is absent is a
/// load-time refusal naming the name and the binary — never a silent run
/// with fewer rules than the author believed they had agreed to.
pub const BUILTIN_PRESETS: &[(&str, &str)] =
    &[("amy/workflow", include_str!("../presets/amy/workflow.yaml"))];

/// The named preset's rule settings, parsed from the compiled-in document.
///
/// The preset is a fragment: it carries `rules:` and nothing else, and the
/// document it extends into keeps every other decision.
pub fn rules(name: &str) -> Result<BTreeMap<String, crate::policy::RuleSetting>> {
    let body = body(name)?;
    let preset: Preset =
        serde_yaml::from_str(body).with_context(|| format!("preset {name} is malformed"))?;
    Ok(preset.rules)
}

/// Every preset this binary answers for, in listing order.
pub fn names() -> Vec<&'static str> {
    BUILTIN_PRESETS.iter().map(|(name, _)| *name).collect()
}

fn body(name: &str) -> Result<&'static str> {
    let Some((_, body)) = BUILTIN_PRESETS.iter().find(|(known, _)| *known == name) else {
        bail!(
            "no preset {name:?} in this binary — presets are part of the catalog, \
             and `sf --version` names the catalog this binary carries; known: {}",
            names().join(", ")
        );
    };
    Ok(body)
}

#[derive(Debug, serde::Deserialize)]
struct Preset {
    #[serde(default)]
    rules: BTreeMap<String, crate::policy::RuleSetting>,
}

/// A preset is a claim about rule ids, and rule ids are a public contract:
/// one that names an id the catalog does not carry would enable nothing,
/// silently, which is the exact failure the preset exists to prevent.
#[cfg(test)]
mod drift {
    use super::*;

    #[test]
    fn every_preset_rule_id_is_a_rule_the_catalog_carries() {
        let catalog = crate::catalog::Catalog::builtin().expect("the built-in catalog loads");
        for (name, _) in BUILTIN_PRESETS {
            for id in rules(name).expect("preset parses").keys() {
                assert!(
                    catalog.get(id).is_some(),
                    "preset {name} enables {id}, which the catalog does not carry"
                );
            }
        }
    }
}
