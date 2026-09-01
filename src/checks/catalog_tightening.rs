//! L2 — a released rule never gets weaker without somebody saying so.
//!
//! `L2.POLICY_ONLY_TIGHTENS` stops a repository loosening its own policy. It
//! cannot see the other direction: the catalog lives in the binary, so upstream
//! can loosen a rule under every consumer at once with no diff downstream.
//!
//! Not hypothetical. Restricting a `text_pattern` entry to one language is both
//! correct and a loosening, and a consumer whose build went red to green after
//! an upgrade cannot tell that from having fixed something.
//!
//! A new rule needs none of this. `Policy::instances` iterates the policy, not
//! the catalog, so a rule nobody enabled never runs — additions are safe by
//! construction and this check is silent about them.

use super::Ctx;
use crate::catalog::Rule;
use crate::fingerprint::{CATALOG_LOCK_PATH, CatalogLock, Reach};
use crate::finding::Finding;
use anyhow::Result;

pub fn run(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    // Nothing committed to compare against. `L5.NO_INERT_RULE` reports that
    // as inertness, which is the honest place for it: silence here would be
    // indistinguishable from agreement.
    let Some(previous) = CatalogLock::load(ctx.root)? else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    for (id, was) in &previous.rules {
        let Some(current) = ctx.catalog.get(id) else {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    CATALOG_LOCK_PATH,
                    format!("catalog-removed:{id}"),
                    format!(
                        "{id} is enabled here and no longer exists in the catalog this sf carries \
                         — every repository that pinned the id lost the rule silently"
                    ),
                )
                .expected(format!("{id} still in the catalog"))
                .actual(format!("absent from catalog {}", short(&catalog_digest_now()))),
            );
            continue;
        };
        let now = Reach::of(current);
        let weakenings = now.weakenings(was);
        if weakenings.is_empty() {
            continue;
        }
        findings.push(
            Finding::new(
                &rule.id,
                rule.severity,
                CATALOG_LOCK_PATH,
                format!("catalog-weakened:{id}"),
                format!(
                    "{id} is weaker than the version this repository locked: {}",
                    weakenings.join("; ")
                ),
            )
            .expected(format!(
                "at least the reach locked against sf {} (catalog {})",
                previous.sf_version,
                short(&previous.catalog_digest)
            ))
            .actual(format!("catalog {}", short(&catalog_digest_now()))),
        );
    }
    Ok(findings)
}

fn catalog_digest_now() -> String {
    crate::fingerprint::catalog_digest()
}

fn short(digest: &str) -> String {
    digest.chars().take(12).collect()
}
