//! Check engines. Each catalog rule names one; the rule's options are data.

pub mod cadence;
pub mod catalog_tightening;
pub mod comment_block;
pub mod command;
pub mod complexity;
pub mod evidence;
pub mod forwarder;
pub mod lock;
pub mod nested;
pub mod shape;
pub mod text_pattern;
pub mod tightening;
pub mod toolchain;

use crate::catalog::{Catalog, CheckKind, Rule};
use crate::finding::Finding;
use crate::policy::{Options, Policy, merge};
use crate::ratchet::Ratchet;
use crate::scan::SourceFile;
use anyhow::{Context, Result};
use std::path::Path;

pub struct Ctx<'a> {
    pub root: &'a Path,
    pub policy: &'a Policy,
    pub catalog: &'a Catalog,
    pub files: &'a [SourceFile],
    pub ratchet: &'a Ratchet,
    /// Repo-relative paths changed by the work under review. `None` means
    /// "unknown", and every gate activates — unknown must never mean skipped.
    pub changed: Option<Vec<String>>,
    /// The git ref the work is measured against, when there is one.
    pub base: Option<String>,
    pub today: String,
    /// Whether `command` rules may actually run. Off unless asked for: a
    /// policy travels with a clone, and `sf check` must be safe on a
    /// repository you have not read.
    pub allow_commands: bool,
    /// The policy governing this run from outside the repository being
    /// checked, when it does. `None` is the vendored case. `Some` means the
    /// policy is an overlay: the rules read the same, but the repository's
    /// own gates, evidence, ratchet and locks are not there, and the checks
    /// that depend on them say inapplicable rather than pass.
    pub overlay: Option<Overlay>,
}

/// An overlay run's provenance: which directory governed it and which exact
/// bytes it carried. The report names both, so a green overlay run cannot be
/// quoted later as a governed one.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Overlay {
    pub path: String,
    pub digest: String,
}

impl Overlay {
    /// The sentence every report format carries.
    pub fn disclaimer(&self) -> String {
        format!(
            "policy overlay: {} (digest {}); this run carried no ratchet and no frozen baseline — it is a report about the code, not evidence the repository is governed",
            self.path, self.digest
        )
    }
}

pub fn options_for(rule: &Rule, policy: &Policy) -> Result<Options> {
    let overrides = policy
        .rules
        .get(&rule.id)
        .map(|s| s.options.clone())
        .unwrap_or(serde_yaml::Value::Null);
    let merged = merge(&rule.defaults, &overrides);
    serde_yaml::from_value(merged)
        .with_context(|| format!("options for rule {} are malformed", rule.id))
}

/// Run every enabled rule instance. Returns findings in stable order.
pub fn run_all(ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (instance, base) in ctx.policy.instances() {
        let Some(rule) = ctx.catalog.get(&base) else {
            anyhow::bail!("policy enables {instance}, but {base} is not a rule in the catalog");
        };
        // An instance whose `when` no longer matches is about a dependency
        // version this repository does not have, so running it would report
        // findings on code that is now right. It is not silently dropped:
        // `L5.NO_INERT_RULE` names it and the range it expected. See
        // `policy::activation`.
        if ctx.policy.activation_of(ctx.root, &instance)?.stale_reason().is_some() {
            continue;
        }
        // An overlay cannot supply repo-local state: the gates read evidence
        // and plans inside the target, and the tightening and catalog rules
        // compare this policy against a baseline the overlay did not bring
        // and the target did not vendor. Silence there reads as coverage, so
        // each one says inapplicable instead — `L5.NO_INERT_RULE`'s argument,
        // one level down.
        if let (Some(overlay), true) = (&ctx.overlay, inapplicable_under_overlay(&base)) {
                findings.push(Finding::new(
                    &instance,
                    rule.severity,
                    format!("{}/policy.yaml", overlay.path),
                    format!("inapplicable:{instance}"),
                    format!(
                        "{instance} needs repo-local state an overlay run does not carry — the policy governing this run lives at {} and the state this rule reads lives in the repository being checked",
                        overlay.path
                    ),
                ));
                continue;
            }
        findings.extend(run_one(&as_instance(rule, &instance), ctx)?);
    }
    Ok(findings)
}

/// Which check kinds read state the policy directory did not bring with it:
/// the gates (evidence, activation paths, plans in `plans/`) and the two
/// baseline comparisons. Everything else — source rules, text patterns,
/// complexity, toolchain, and the cadence modes over the repository's own
/// documents — reads the code and the policy alone and runs unchanged.
fn inapplicable_under_overlay(base: &str) -> bool {
    matches!(base, "L3.GATE_HAS_FRESH_EVIDENCE" | "L2.POLICY_ONLY_TIGHTENS" | "L2.FACTORY_CONFIG_IS_LOCKED")
}

/// The same rule under an instance's name, so its findings, its options and
/// its ratchet entries stay separate from the other instances'.
pub fn as_instance(rule: &Rule, instance: &str) -> Rule {
    let mut copy = rule.clone();
    copy.id = instance.to_string();
    copy
}

pub fn run_one(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    // Repo-local rules say inapplicable under an overlay rather than run to
    // a misleading missing-manifest finding. `--rule` goes through here too,
    // so an operator asking for one of these by name gets the same answer.
    if let Some(overlay) = &ctx.overlay {
        let base = crate::policy::base_rule_id(&rule.id);
        if inapplicable_under_overlay(base) {
            return Ok(vec![Finding::new(
                &rule.id,
                rule.severity,
                format!("{}/policy.yaml", overlay.path),
                format!("inapplicable:{}", rule.id),
                format!(
                    "{base} needs repo-local state an overlay run does not carry — the policy governing this run lives at {}, and the evidence, ratchet and locks this rule reads live in the repository being checked",
                    overlay.path
                ),
            )]);
        }
    }
    let opts = options_for(rule, ctx.policy)?;
    match &rule.check {
        // The kinds that read source: a grammar or a regex over the files the
        // scope selects.
        CheckKind::Shape { languages } => shape::run(rule, &opts, languages, ctx),
        CheckKind::Nested { languages } => nested::run(rule, &opts, languages, ctx),
        CheckKind::Complexity => complexity::run(rule, &opts, ctx),
        CheckKind::CommentBlock => comment_block::run(rule, &opts, ctx),
        CheckKind::Forwarder { languages } => forwarder::run(rule, &opts, languages, ctx),
        CheckKind::TextPattern => text_pattern::run(rule, &opts, ctx),
        // The kinds that read what the repository committed about itself.
        bookkeeping => run_bookkeeping(bookkeeping, rule, &opts, ctx),
    }
}

/// Kinds whose subject is the repository's own configuration, evidence and
/// generated artifacts rather than its source code.
///
/// Split from `run_one` because a flat dispatch reached
/// `L1.COMPLEXITY_CEILING` the moment one more kind existed. A dispatch table
/// carries no branching a reader must hold, so the ceiling firing here is the
/// metric's limitation, not a defect — see the grain plan, shipped
/// in `8cc43fb`. The seam chosen means
/// something (source versus bookkeeping) rather than merely reaching twelve.
fn run_bookkeeping(
    kind: &CheckKind,
    rule: &Rule,
    opts: &Options,
    ctx: &Ctx,
) -> Result<Vec<Finding>> {
    match kind {
        CheckKind::Lock => lock::run(rule, opts, ctx),
        CheckKind::Expiry => lock::expiry(rule, ctx),
        CheckKind::Cadence { mode } => cadence::run(rule, opts, ctx, *mode),
        CheckKind::Evidence => evidence::run(rule, opts, ctx),
        CheckKind::Toolchain => toolchain::run(rule, opts, ctx),
        CheckKind::PolicyTightening => tightening::run(rule, opts, ctx),
        CheckKind::CatalogTightening => catalog_tightening::run(rule, ctx),
        CheckKind::Command => command::run(rule, opts, ctx),
        // Handled by `run_one` before it delegates here. Not `unreachable!`:
        // an added kind should reach its own arm, not abort the run.
        CheckKind::Shape { .. }
        | CheckKind::Nested { .. }
        | CheckKind::Complexity
        | CheckKind::TextPattern
        | CheckKind::CommentBlock
        | CheckKind::Forwarder { .. } => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod overlay {
    use super::{inapplicable_under_overlay, Ctx};
    use crate::catalog::Catalog;
    use crate::policy::{FIXTURES_DIR, Policy};
    use crate::ratchet::Ratchet;
    use crate::scan;

    /// The gate evidence rule needs a manifest, activation paths and a plan
    /// inside the repository being checked. Under an overlay none of that is
    /// there, and a run that read nothing must not render as a rule that
    /// found nothing — it reports inapplicable with the overlay's path.
    #[test]
    fn a_repo_local_rule_reports_inapplicable_under_an_overlay() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES_DIR)
            .join("L3.GATE_HAS_FRESH_EVIDENCE");
        let policy: Policy = serde_yaml::from_str(
            "version: 1\nproject:\n  name: overlay-target\n  languages: [python]\nrules:\n  L3.GATE_HAS_FRESH_EVIDENCE:\n    enabled: true\n  L1.NO_BLANKET_SUPPRESSION:\n    enabled: true\n",
        )
        .expect("the policy parses");
        let catalog = Catalog::builtin().expect("the built-in catalog loads");
        let files = scan::walk(&root, &policy).expect("the fixture repo scans");
        let ratchet = Ratchet::default();
        let ctx = Ctx {
            root: &root,
            policy: &policy,
            catalog: &catalog,
            files: &files,
            ratchet: &ratchet,
            changed: None,
            base: None,
            today: crate::clock::today(),
            allow_commands: false,
            overlay: Some(super::Overlay {
                path: "../factory-policy/.software-factory".to_string(),
                digest: "0000".to_string(),
            }),
        };
        let findings = super::run_all(&ctx).expect("the overlay run completes");
        let gate = findings
            .iter()
            .find(|f| f.rule == "L3.GATE_HAS_FRESH_EVIDENCE")
            .expect("the gate rule must say inapplicable rather than run");
        assert!(gate.key.starts_with("inapplicable:"), "{}", gate.key);
        assert!(
            gate.message.contains("repo-local state"),
            "names what the run does not carry: {}",
            gate.message
        );
        assert!(
            gate.message.contains("../factory-policy/.software-factory"),
            "names the policy that governed the run: {}",
            gate.message
        );
        // The source rule ran for real, not as inapplicable — whatever it
        // found, it went through the ordinary path, and its absence of
        // findings is silence from a rule that ran, not a rule that could
        // not.
        assert!(
            !findings.iter().any(|f| f.rule == "L1.NO_BLANKET_SUPPRESSION"
                && f.key.starts_with("inapplicable:")),
            "a source rule is unaffected by an overlay: {findings:?}"
        );
    }

    #[test]
    fn the_overlay_set_names_only_the_rules_that_need_repo_local_state() {
        for id in ["L3.GATE_HAS_FRESH_EVIDENCE", "L2.POLICY_ONLY_TIGHTENS", "L2.FACTORY_CONFIG_IS_LOCKED"] {
            assert!(inapplicable_under_overlay(id), "{id} reads repo-local state");
        }
        for id in ["L1.COMPLEXITY_CEILING", "L1.NO_BLANKET_SUPPRESSION", "L4.DOC_LINKS_RESOLVE", "L5.NO_INERT_RULE"] {
            assert!(
                !inapplicable_under_overlay(id),
                "{id} reads the code and the policy alone, and runs unchanged under an overlay"
            );
        }
    }
}
