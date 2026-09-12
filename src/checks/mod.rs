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

use crate::catalog::{Catalog, CadenceMode, CheckKind, Rule};
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
        // An overlay cannot supply repo-local state: the gates read
        // evidence inside the target, the tightening rules compare against
        // a baseline the overlay did not bring. Silence there reads as
        // coverage, so each one says inapplicable instead — and which
        // rules those are is derived from the check kind, so a rule added
        // tomorrow answers for itself.
        if let (Some(overlay), Some(reason)) = (
            &ctx.overlay,
            inapplicable_under_overlay(&rule.check),
        ) {
            findings.push(inapplicable_finding(rule, &instance, overlay, reason));
            continue;
        }
        findings.extend(run_one(&as_instance(rule, &instance), ctx)?);
    }
    Ok(findings)
}

/// Derived from the check kind, never from a list of ids: ids somebody
/// thought of go stale on the next rule, and what these rules share is
/// structural — the state they read lives beside the policy, not the code.
///
/// A rule belongs here when the files its check answers for exist only
/// because *this tool* acted on the repository being checked: locks,
/// ratchets, evidence, fixtures, the catalog fingerprint, the root
/// allowlist. A missing file there is state the target never held, not a
/// violation the code earned; running anyway would render that silence as
/// coverage.
///
/// Kinds whose subject is the code — queries, regexes, complexity,
/// toolchain, commands this policy brings — read the code and the policy
/// alone, and run unchanged.
fn inapplicable_under_overlay(check: &CheckKind) -> Option<&'static str> {
    let reason = match check {
        // A gate's evidence manifest and activation digests live inside the
        // repository being checked, sealed by `sf seal` against runs of its
        // own code. An overlay run is expressly not evidence, and it cannot
        // conjure a manifest the target never sealed.
        CheckKind::Evidence => {
            "gate evidence is sealed inside the repository being checked"
        }
        // Locks, the ratchet that freezes against them and the dated
        // exceptions it carries: all written by this tool into the target's
        // factory directory, which an overlay run may not write to and the
        // target does not carry. `sf lock` is refused under an overlay by
        // design, so nothing can ever clear a finding here — a finding
        // whose only fix is refused is not advice.
        CheckKind::Lock | CheckKind::Expiry => {
            "locks and the frozen baseline live in the repository being checked"
        }
        // The previous policy and the ratchet are compared against the
        // target's git history or a baseline directory vendored beside
        // them. An overlay cannot read the target's history as a
        // statement about the overlay's own direction, and no run it
        // governs has a "previous" to compare against.
        CheckKind::PolicyTightening => {
            "the previous policy is compared against history this run does not carry"
        }
        // The catalog fingerprint records which catalog the target agreed
        // to. The overlay's target never agreed to one — the agreement
        // lives in the repository the overlay comes from.
        CheckKind::CatalogTightening => {
            "the catalog fingerprint is an agreement this repository never made"
        }
        CheckKind::Cadence { mode } => cadence_inapplicable_under_overlay(*mode)?,
        // Kinds that read the code: they run unchanged, because an overlay
        // is a report about the code, not about the governance that
        // surrounds it.
        _ => return None,
    };
    Some(reason)
}

/// The cadence modes split by what their subject is. A mode whose subject is
/// a document the repository holds about its own rules, plans or root
/// layout travels with the factory directory; a mode whose subject is the
/// repository's own documentation and links runs wherever the docs do.
fn cadence_inapplicable_under_overlay(mode: CadenceMode) -> Option<&'static str> {
    let reason = match mode {
        // The prose explaining each rule, and the fixtures proving each
        // check fires, are held by the repository the policy lives in.
        // Reading the target for them misreports every enabled rule as
        // unexplained and unproven — the prose and the fixtures are in the
        // overlay's own repository, provably, by [`crate::verify`].
        CadenceMode::RuleCitations => {
            "the prose explaining a rule lives beside the policy, not beside the code"
        }
        CadenceMode::MutationCoverage => {
            "mutation fixtures are held by the repository the policy lives in"
        }
        // `sf init` writes the root allowlist into a repository; clearing a
        // finding here means writing a file the overlay is not allowed to
        // write, and a finding whose only fix is refused is not advice.
        CadenceMode::RootFiles => {
            "the root allowlist is a file this tool writes into the repository being checked"
        }
        // The execution order and the exit conditions it enforces are the
        // target's own queue of undone work. A policy that governs from
        // outside carries no plans in the target, so a plan that is
        // "not in the order" here is a plan that was never the target's.
        CadenceMode::PlanCadence => {
            "plans and their execution order live in the repository being checked"
        }
        // A gate's criteria document lives beside the gate's evidence, in
        // the repository that declares the gate — which, for an overlay, is
        // the overlay's own repository, not the target.
        CadenceMode::GateCoverage => {
            "a gate's criteria document lives beside the gate, not beside the code"
        }
        // Documentation links, plan criteria markers, proof budgets and
        // claim citations are statements about documents the target does
        // hold; they read the same wherever the policy lives. InertRules
        // composes (see [`inert_rules`]) rather than joins this set.
        CadenceMode::DocLinks
        | CadenceMode::PlanCriteria
        | CadenceMode::PlanProofBudget
        | CadenceMode::ClaimCitations
        | CadenceMode::GatePlanPlacement
        | CadenceMode::InertRules
        | CadenceMode::RuleCommands => return None,
    };
    Some(reason)
}

/// The finding an overlay run renders for a rule it cannot honestly run.
/// One shape for both call sites, so `--rule` asking by name and a full
/// `run_all` cannot drift apart in what they report.
pub fn inapplicable_finding(
    rule: &Rule,
    instance: &str,
    overlay: &Overlay,
    reason: &str,
) -> Finding {
    Finding::new(
        instance,
        rule.severity,
        format!("{}/policy.yaml", overlay.path),
        format!("inapplicable:{instance}"),
        format!(
            "{instance} needs repo-local state an overlay run does not carry — the policy governing this run lives at {} ({reason})",
            overlay.path
        ),
    )
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
    if let (Some(overlay), Some(reason)) =
        (&ctx.overlay, inapplicable_under_overlay(&rule.check))
    {
        return Ok(vec![inapplicable_finding(rule, &rule.id, overlay, reason)]);
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
    use super::{Ctx, Rule, inapplicable_under_overlay};
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
        let catalog = Catalog::builtin().expect("the built-in catalog loads");
        for (id, rule) in &catalog.rules {
            let why = inapplicable_under_overlay(&rule.check).is_some();
            let needs = matches!(
                id.as_str(),
                // Gate evidence: sealed manifests, activation digests.
                "L3.GATE_HAS_FRESH_EVIDENCE"
                // Locks, the ratchet and its dated exceptions.
                | "L2.DEPENDENCIES_CHANGE_DELIBERATELY"
                | "L2.FACTORY_CONFIG_IS_LOCKED"
                | "L2.GENERATED_FILES_ARE_LOCKED"
                | "L2.NO_PERMANENT_EXCEPTION"
                // Direction and catalog comparisons against a baseline.
                | "L2.POLICY_ONLY_TIGHTENS"
                | "L2.CATALOG_ONLY_TIGHTENS"
                // Cadence over factory state: prose, fixtures, allowlist,
                // plans queue, gate criteria document.
                | "L4.EVERY_RULE_HAS_A_WHY"
                | "L5.EVERY_CHECK_HAS_A_MUTATION_TEST"
                | "L4.ROOT_FILES_ARE_DECLARED"
                | "L4.PLAN_DECLARES_EXIT_CONDITION"
                | "L3.GATE_COVERS_THE_PLAN"
            );
            assert_eq!(
                why, needs,
                "{id} (a {:?} check) {}",
                rule.check,
                if needs { "reads repo-local state" } else { "reads the code and the policy alone" }
            );
            // Every reason the set can produce is a sentence, not a bare id:
            // the finding is the report, and a finding naming only the rule
            // it cannot run is indistinguishable from one that failed.
            if let Some(reason) = inapplicable_under_overlay(&rule.check) {
                assert!(!reason.is_empty(), "{id} needs a reason a reader can act on");
            }
        }
    }

    /// The set is derived from the check kind, so a rule nobody has
    /// written yet answers for itself: a synthetic `cadence { mode:
    /// root_files }` rule, under an id the catalog has never carried, is
    /// inapplicable by its kind alone. Removing `RootFiles` from the
    /// derived set makes this fail while every catalog rule above still
    /// passes — the drift a list of ids could not catch.
    #[test]
    fn a_rule_nobody_has_written_yet_answers_for_itself() {
        let synthetic = serde_yaml::from_str::<Rule>(
            "id: L9.SYNTHETIC_ROOT_ALLOWLIST\nlayer: L4\ntitle: A rule not in the catalog\n\
             severity: low\nstatement: >-\n  The synthetic statement.\nwhy: >-\n  Because a synthetic rule needs one.\n\
             fix: >-\n  Fix it synthetically.\ncheck:\n  kind: cadence\n  mode: root_files\ndefaults: {}\n",
        )
        .expect("the synthetic rule parses");
        let reason = inapplicable_under_overlay(&synthetic.check)
            .expect("a root_files rule is inapplicable under an overlay");
        assert!(
            reason.contains("root allowlist"),
            "carries the decided reasoning, not just the verdict: {reason}"
        );
    }

    /// The bare suppression marker, built char-by-char so this file does
    /// not carry the pattern it is a fixture for.
    const BARE_NOQA: &str = concat!("#", " noqa");

    /// A scratch target this test can build: one Python file that earns a
    /// real suppression finding, nothing else. The same policy then runs
    /// twice over it — vendored into the root, and as an overlay from
    /// outside — and the code findings must be identical, because an
    /// overlay is a report about the code, and the code has not changed.
    struct ScratchTarget {
        root: std::path::PathBuf,
        overlay_dir: std::path::PathBuf,
        /// Scanned once per target; every ctx shares the same walk, which is
        /// what an overlay run and its vendored twin have to see.
        files: Vec<scan::SourceFile>,
    }

    impl ScratchTarget {
        fn new(tag: &str, policy: &Policy) -> Self {
            let base = std::env::temp_dir().join(format!("sf-{tag}-{}", std::process::id()));
            let root = base.join("target-repo");
            let overlay_dir = base.join("governing-repo").join(".software-factory");
            std::fs::create_dir_all(root.join("src")).expect("the target's source is created");
            std::fs::create_dir_all(&overlay_dir).expect("the overlay directory is created");
            std::fs::write(root.join("src").join("app.py"), format!("import os  {}\n", BARE_NOQA))
                .expect("the target's source is written");
            std::fs::write(
                overlay_dir.join("policy.yaml"),
                serde_yaml::to_string(policy).expect("the policy serialises"),
            )
            .expect("the overlay's policy is written");
            let files = scan::walk(&root, policy).expect("the target scans");
            Self { root, overlay_dir, files }
        }

        /// The overlay form: `run_all` with `overlay` set, ratchet empty.
        fn overlay_ctx<'a>(&'a self, catalog: &'a Catalog, policy: &'a Policy) -> Ctx<'a> {
            Ctx {
                root: &self.root,
                policy,
                catalog,
                files: &self.files,
                ratchet: &RATCHET_PLACEHOLDER,
                changed: None,
                base: None,
                today: crate::clock::today(),
                allow_commands: false,
                overlay: Some(super::Overlay {
                    path: self.overlay_dir.display().to_string(),
                    digest: "0000".to_string(),
                }),
            }
        }

        /// The vendored form: the same policy bytes inside the target,
        /// ratchet empty here; the frozen variant passes its own.
        fn vendored_ctx<'a>(
            &'a self,
            catalog: &'a Catalog,
            policy: &'a Policy,
            ratchet: &'a Ratchet,
        ) -> Ctx<'a> {
            Ctx {
                root: &self.root,
                policy,
                catalog,
                files: &self.files,
                ratchet,
                changed: None,
                base: None,
                today: crate::clock::today(),
                allow_commands: false,
                overlay: None,
            }
        }

        /// Vendor the policy into the target and re-walk, so the vendored
        /// run sees the factory directory the target now carries — the
        /// same repository, governed from inside instead of outside.
        fn vendor_policy(&mut self, policy: &Policy) {
            let factory = self.root.join(".software-factory");
            std::fs::create_dir_all(&factory).expect("the vendored factory directory is created");
            std::fs::write(
                factory.join("policy.yaml"),
                serde_yaml::to_string(policy).expect("the policy serialises"),
            )
            .expect("the vendored policy is written");
            self.files = scan::walk(&self.root, policy).expect("the vendored target scans");
        }
    }

    /// The empty ratchet the plain vendored ctx runs under, mirroring
    /// `check_vendored`'s behaviour only when the target's factory
    /// directory carries no ratchet of its own.
    static RATCHET_PLACEHOLDER: Ratchet = Ratchet {
        version: 1,
        rules: std::collections::BTreeMap::new(),
    };

    /// The policy every scratch-target test shares: one real source rule the
    /// target's code violates, one factory-state rule that goes inapplicable
    /// under an overlay and inert vendored, the meta rule, and the root
    /// allowlist rule the plan's judgment call decided on.
    fn shared_policy() -> Policy {
        serde_yaml::from_str(
            "version: 1\nproject:\n  name: overlay-target\n  languages: [python]\n\
             rules:\n  L1.NO_BLANKET_SUPPRESSION:\n    enabled: true\n  L2.CATALOG_ONLY_TIGHTENS:\n    enabled: true\n  L5.NO_INERT_RULE:\n    enabled: true\n  L4.ROOT_FILES_ARE_DECLARED:\n    enabled: true\n",
        )
        .expect("the shared policy parses")
    }

    /// The identity an overlay run must hold, in three partitions. The
    /// code rules report the same findings wherever the policy lives,
    /// because an overlay is a report about the code and the code has not
    /// changed.
    ///
    /// The factory-state rules report their real findings vendored and say
    /// `inapplicable:` under the overlay: same policy, same code, one
    /// honest sentence about what the run cannot know. The meta rule
    /// composes — inert vendored, silent about it under the overlay.
    #[test]
    fn overlay_matches_vendored_over_the_same_code() {
        let policy = shared_policy();
        let mut target = ScratchTarget::new("overlay-vendored", &policy);
        let catalog = Catalog::builtin().expect("the built-in catalog loads");

        let overlay_findings = super::run_all(&target.overlay_ctx(&catalog, &policy))
            .expect("the overlay run completes");
        target.vendor_policy(&policy);
        let vendored_findings = super::run_all(
            &target.vendored_ctx(&catalog, &policy, &RATCHET_PLACEHOLDER),
        )
        .expect("the vendored run completes");

        // Partition A: findings the code earned. Identical in the four
        // fields a reader or a report can see — same rule, same location,
        // same stable key, same message — wherever the policy lives.
        let earned = |findings: &[crate::finding::Finding]| {
            findings
                .iter()
                .filter(|f| f.rule == "L1.NO_BLANKET_SUPPRESSION")
                .map(|f| (f.rule.clone(), f.location.clone(), f.key.clone(), f.message.clone()))
                .collect::<Vec<_>>()
        };
        let overlay_earned = earned(&overlay_findings);
        let vendored_earned = earned(&vendored_findings);
        assert!(
            !overlay_earned.is_empty(),
            "the scratch target must earn at least one real finding: {overlay_findings:?}"
        );
        assert_eq!(
            overlay_earned, vendored_earned,
            "the code under check is the same, so the findings it earns are the same"
        );

        // Partition B: the factory-state rules. Under the overlay each
        // says inapplicable, naming the state it cannot know; vendored,
        // each runs and reports what the target really holds. Every extra
        // finding the overlay carries is an `inapplicable:` one — anything
        // else is a rule that ran to a misleading finding, the defect the
        // plan measured.
        let overlay_inapplicable: Vec<_> = overlay_findings
            .iter()
            .filter(|f| f.key.starts_with("inapplicable:"))
            .collect();
        assert!(
            !overlay_inapplicable.is_empty(),
            "the policy enables factory-state rules, so the overlay must say what it cannot know"
        );
        for finding in &overlay_inapplicable {
            let rule = catalog
                .get(crate::policy::base_rule_id(&finding.rule))
                .expect("an inapplicable finding names a rule in the catalog");
            assert!(
                inapplicable_under_overlay(&rule.check).is_some(),
                "{} reported inapplicable but its check ({:?}) runs unchanged under an overlay",
                finding.rule, rule.check
            );
            assert!(
                finding.message.contains("repo-local state"),
                "names what the run does not carry: {:?}",
                finding.message
            );
            // The vendored twin never says inapplicable. It either reports
            // a real finding (the missing allowlist) or stays silent in the
            // ordinary way an engine does (no fingerprint to compare
            // against, which the meta rule then reports as inertness) —
            // either is the rule honestly running, never one that cannot.
            let vendored_rule_findings: Vec<_> = vendored_findings
                .iter()
                .filter(|f| f.rule == finding.rule)
                .collect();
            assert!(
                vendored_rule_findings
                    .iter()
                    .all(|f| !f.key.starts_with("inapplicable:")),
                "vendored, {} never says inapplicable",
                finding.rule
            );
            if vendored_rule_findings.is_empty() {
                // The engine ran and found nothing, so the meta rule owes
                // an inertness finding about this very rule: silence plus
                // a silent meta rule would be a rule lying about running.
                assert!(
                    vendored_findings.iter().any(|f| f.rule == "L5.NO_INERT_RULE"
                        && f.message.contains(finding.rule.as_str())),
                    "vendored and silent, {} must be reported inert by the meta rule",
                    finding.rule
                );
            }
        }

        // The extras over the code findings are exactly the inapplicable
        // ones: nothing else may differ between the two runs.
        let extras: Vec<_> = overlay_findings
            .iter()
            .filter(|f| !vendored_findings.iter().any(|v|
                v.rule == f.rule && v.key == f.key && v.location == f.location))
            .collect();
        assert!(
            extras.iter().all(|f| f.key.starts_with("inapplicable:")),
            "an overlay run may differ from the vendored one only by saying what it cannot know: {extras:?}"
        );

        // Partition M: the meta rule. Vendored, no fingerprint, so
        // L2.CATALOG_ONLY_TIGHTENS is genuinely inert and is reported as
        // exactly that. Under the overlay the same rule already said what
        // it cannot know, and the meta rule says nothing about it — two
        // findings about one silence would be one too many.
        let vendored_inert = vendored_findings
            .iter()
            .find(|f| f.rule == "L5.NO_INERT_RULE"
                && f.message.contains("L2.CATALOG_ONLY_TIGHTENS"))
            .expect("vendored, the fingerprint-less catalog rule is inert");
        assert!(vendored_inert.message.contains("fingerprint"), "{}", vendored_inert.message);
        assert!(
            !overlay_findings.iter().any(|f| f.rule == "L5.NO_INERT_RULE"
                && f.message.contains("L2.CATALOG_ONLY_TIGHTENS")),
            "under the overlay, the meta rule skips the rule that already said what it cannot know"
        );
        let _ = std::fs::remove_dir_all(&target.root);
    }

    /// The ratchet is the one difference the plan allows. Vendored, a
    /// repository may freeze the finding its code earned and adopt the
    /// rule; an overlay run carries no ratchet by construction, so the
    /// same finding stays live. Both runs happened over identical code —
    /// the difference is the baseline, nothing else.
    #[test]
    fn the_ratchet_is_the_only_difference_an_overlay_carries() {
        let policy = shared_policy();
        let mut target = ScratchTarget::new("overlay-ratchet", &policy);
        let catalog = Catalog::builtin().expect("the built-in catalog loads");

        let overlay_findings = super::run_all(&target.overlay_ctx(&catalog, &policy))
            .expect("the overlay run completes");
        let live = overlay_findings
            .iter()
            .find(|f| f.rule == "L1.NO_BLANKET_SUPPRESSION"
                && !f.key.starts_with("inapplicable:"))
            .expect("the target's code earns the suppression finding in both runs");

        // The vendored run with a ratchet freezing exactly that key: the
        // adoption shape `sf ratchet` writes.
        target.vendor_policy(&policy);
        let mut ratchet = Ratchet::default();
        let mut frozen_keys = std::collections::BTreeSet::new();
        frozen_keys.insert(live.key.clone());
        ratchet.seed(
            "L1.NO_BLANKET_SUPPRESSION",
            frozen_keys,
            "2027-01-01",
            None,
        );
        let raw = super::run_all(&target.vendored_ctx(&catalog, &policy, &ratchet))
            .expect("the vendored run completes");
        let (frozen_live, frozen_count) = ratchet.apply(raw);
        assert_eq!(
            frozen_count, 1,
            "the ratchet froze exactly the finding the code earned"
        );
        assert!(
            !frozen_live
                .iter()
                .any(|f| f.rule == "L1.NO_BLANKET_SUPPRESSION"
                    && !f.key.starts_with("inapplicable:")),
            "frozen: the same finding the overlay run reports live"
        );
        let _ = std::fs::remove_dir_all(&target.root);
    }

    /// `L5.NO_INERT_RULE` composes with the set rather than joining it: a
    /// rule already saying inapplicable is silent for a stated reason, and
    /// reporting it *also* inert would be two findings about one silence.
    /// The mirror direction holds too: the same rule over the same code,
    /// vendored, is genuinely inert — `L2.CATALOG_ONLY_TIGHTENS` over a
    /// root with no catalog fingerprint — and is reported as exactly that.
    #[test]
    fn inertness_skips_a_rule_that_already_said_what_it_cannot_know() {
        let policy = shared_policy();
        let mut target = ScratchTarget::new("overlay-inert", &policy);
        let catalog = Catalog::builtin().expect("the built-in catalog loads");

        let overlay_findings = super::run_all(&target.overlay_ctx(&catalog, &policy))
            .expect("the overlay run completes");
        let inapplicable = overlay_findings
            .iter()
            .find(|f| f.rule == "L2.CATALOG_ONLY_TIGHTENS" && f.key.starts_with("inapplicable:"))
            .expect("the catalog rule says inapplicable under the overlay");
        let reported_inert = overlay_findings.iter().any(|f| {
            f.rule == "L5.NO_INERT_RULE" && f.message.contains("L2.CATALOG_ONLY_TIGHTENS")
        });
        assert!(
            !reported_inert,
            "a rule that said what it cannot know is not also inert: {inapplicable:?}"
        );

        // The vendored half: no overlay, no fingerprint, so the same rule
        // has nothing to compare against — and that is inertness, reported
        // as inertness, not as inapplicability.
        target.vendor_policy(&policy);
        let vendored_findings = super::run_all(
            &target.vendored_ctx(&catalog, &policy, &RATCHET_PLACEHOLDER),
        )
        .expect("the vendored run completes");
        let inert = vendored_findings
            .iter()
            .find(|f| f.rule == "L5.NO_INERT_RULE"
                && f.message.contains("L2.CATALOG_ONLY_TIGHTENS"))
            .expect("the same rule, vendored and with no fingerprint, is reported inert");
        assert!(
            inert.message.contains("fingerprint"),
            "names the missing state: {}",
            inert.message
        );
        assert!(
            !vendored_findings.iter().any(|f| {
                f.rule == "L2.CATALOG_ONLY_TIGHTENS" && f.key.starts_with("inapplicable:")
            }),
            "vendored, the rule runs and reports inertness, never inapplicability"
        );
        let _ = std::fs::remove_dir_all(&target.root);
    }

    /// `L4.ROOT_FILES_ARE_DECLARED` is the plan's judgment call: a target
    /// *could* carry `.allowed-root-files`, but clearing the finding means
    /// writing into a repository the overlay is not allowed to write to,
    /// and a finding whose only fix is refused is not advice. Vendored,
    /// the same missing allowlist is a real finding: the repository owns
    /// the fix.
    #[test]
    fn the_root_allowlist_rule_is_inapplicable_because_its_only_fix_is_refused() {
        let policy = shared_policy();
        let mut target = ScratchTarget::new("overlay-rootfiles", &policy);
        let catalog = Catalog::builtin().expect("the built-in catalog loads");

        let overlay_findings = super::run_all(&target.overlay_ctx(&catalog, &policy))
            .expect("the overlay run completes");
        let inapplicable = overlay_findings
            .iter()
            .find(|f| f.rule == "L4.ROOT_FILES_ARE_DECLARED"
                && f.key.starts_with("inapplicable:"))
            .expect("the root-files rule says inapplicable under the overlay");
        assert!(
            inapplicable.message.contains("root allowlist"),
            "carries the decided reasoning: {}",
            inapplicable.message
        );

        // Vendored, the same target earns a missing-allowlist finding, not
        // an inapplicable one: the repository owns the fix.
        target.vendor_policy(&policy);
        let vendored_findings = super::run_all(
            &target.vendored_ctx(&catalog, &policy, &RATCHET_PLACEHOLDER),
        )
        .expect("the vendored run completes");
        assert!(
            vendored_findings.iter().any(|f| f.rule == "L4.ROOT_FILES_ARE_DECLARED"
                && f.key == "missing-allowlist"),
            "vendored, the missing allowlist is a real finding: {vendored_findings:?}"
        );
        let _ = std::fs::remove_dir_all(&target.root);
    }
}
