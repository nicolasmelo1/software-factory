//! L4 — how documentation, plans and rules stay attached to each other.
//!
//! These are the cheapest checks in the tool and the ones a greenfield repo
//! should turn on first: they cost three markdown files and they are what
//! keeps the other layers from drifting into folklore.

use super::Ctx;
use crate::catalog::{CadenceMode, CheckKind, Rule};
use crate::finding::Finding;
use crate::lang::Lang;
use crate::policy::{FIXTURES_DIR, Options};
use crate::scan;
use anyhow::Result;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx, mode: CadenceMode) -> Result<Vec<Finding>> {
    match mode {
        CadenceMode::DocLinks => doc_links(rule, opts, ctx),
        CadenceMode::RootFiles => root_files(rule, opts, ctx),
        CadenceMode::RuleCitations => rule_citations(rule, opts, ctx),
        CadenceMode::PlanCadence => plan_cadence(rule, opts, ctx),
        CadenceMode::PlanCriteria => plan_criteria(rule, opts, ctx),
        CadenceMode::PlanProofBudget => plan_proof_budget(rule, opts, ctx),
        CadenceMode::GateCoverage => gate_coverage(rule, ctx),
        CadenceMode::MutationCoverage => mutation_coverage(rule, ctx),
        CadenceMode::InertRules => inert_rules(rule, ctx),
        CadenceMode::RuleCommands => rule_commands(rule, ctx),
        CadenceMode::ClaimCitations => claim_citations(rule, opts, ctx),
    }
}

fn covers_a_declared_language<'a>(
    mut languages: impl Iterator<Item = &'a String>,
    ctx: &Ctx,
) -> bool {
    languages.any(|name| ctx.policy.project.languages.contains(name))
}

/// What a rule nobody pointed at anything owes: configuration, or an
/// admission in writing.
const CONFIGURE_OR_DISABLE: &str = "a configured rule, or an honest `enabled: false`";

/// What a rule whose `when` went false owes instead. There is nothing to
/// configure: the rule is about a dependency version this repository does not
/// have, so the answers are to repoint it or to delete it along with the
/// version it described.
const REPOINT_OR_REMOVE: &str =
    "a rule repointed at the version this repository installs, or removed with it";

/// An enabled rule pointed at nothing. It passes every run and reads exactly
/// like a rule that is protecting you.
fn inert_rules(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (instance, base) in ctx.policy.instances() {
        let Some(candidate) = ctx.catalog.get(&base) else {
            continue;
        };
        let (id, candidate) = (&instance, &super::as_instance(candidate, &instance));
        // A `when` that no longer matches comes first: it explains why the
        // rule is silent better than anything about its scope would, and it
        // is the one inertness a repository can walk into by changing a
        // dependency rather than by editing the policy.
        let activation = ctx.policy.activation_of(ctx.root, id)?;
        let (reason, expected) = match activation.stale_reason() {
            Some(reason) => (Some(reason.to_string()), REPOINT_OR_REMOVE),
            None => (inert_reason(candidate, ctx)?, CONFIGURE_OR_DISABLE),
        };
        let Some(reason) = reason else { continue };
        findings.push(
            Finding::new(
                &rule.id,
                rule.severity,
                crate::policy::POLICY_PATH,
                format!("inert:{id}"),
                format!("{id} is enabled but cannot produce a finding — {reason}"),
            )
            .expected(expected),
        );
    }
    Ok(findings)
}

/// Why an enabled rule can never produce a finding in this repository, if any.
///
/// Split the way `checks::run_one` splits: grammar-reading kinds here,
/// bookkeeping kinds next door. Lifting it out of `inert_rules`'s loop keeps
/// reason and reporting separately readable, and covering one more kind then
/// costs only the half that grew. That is `L1.COMPLEXITY_CEILING`'s argument:
/// the two L3 arms below pushed the single function that held all of this to
/// 14 paths against a ceiling of 12.
fn inert_reason(candidate: &Rule, ctx: &Ctx) -> Result<Option<String>> {
    let options = super::options_for(candidate, ctx.policy)?;
    let reason: Option<String> = match &candidate.check {
        // Three arms and one message: `Shape`, `Nested` and `Forwarder` carry
        // different query types, so the bindings cannot be merged into a
        // single or-pattern even though the question and the answer are
        // identical.
        CheckKind::Shape { languages }
            if !covers_a_declared_language(languages.keys(), ctx) =>
        {
            Some("no query for any language this repository declares".to_string())
        }
        CheckKind::Nested { languages }
            if !covers_a_declared_language(languages.keys(), ctx) =>
        {
            Some("no query for any language this repository declares".to_string())
        }
        CheckKind::Forwarder { languages }
            if !covers_a_declared_language(languages.keys(), ctx) =>
        {
            Some("no query for any language this repository declares".to_string())
        }
        CheckKind::TextPattern => {
            if scan::select(ctx.files, &options.scope, &options.exclude)?.is_empty() {
                Some("scope matches no file in this repository: it can never see a line to check".to_string())
            } else {
                None
            }
        }
        CheckKind::Complexity => {
            if !any_parseable_declared_file(ctx, &options)? {
                Some(
                    "no file in scope parses into a language this repository declares: the ceiling can never be evaluated".to_string(),
                )
            } else {
                None
            }
        }
        bookkeeping => inert_bookkeeping_reason(bookkeeping, &options, ctx)?,
    };
    Ok(reason)
}

/// The same question for the kinds that read configuration rather than source.
///
/// The trailing `_ => None` is the known gap, and the compiler cannot ask for
/// the next one: `expiry`, `policy_tightening` and every `cadence` mode other
/// than `gate_coverage` still have no inertness test.
fn inert_bookkeeping_reason(check: &CheckKind, options: &Options, ctx: &Ctx) -> Result<Option<String>> {
    Ok(match check {
        CheckKind::Lock if options.scope.is_empty() => {
            Some("no scope: it locks nothing".to_string())
        }
        CheckKind::Command if options.run.is_none() => {
            Some("no command set: there is nothing for it to run".to_string())
        }
        CheckKind::Toolchain => inert_toolchain_reason(options, ctx),
        CheckKind::Evidence => inert_evidence_reason(ctx),
        CheckKind::Cadence { mode: CadenceMode::GateCoverage } => inert_gate_coverage_reason(ctx),
        CheckKind::Cadence { mode: CadenceMode::PlanProofBudget }
            if scan::select(ctx.files, &options.scope, &options.exclude)?.is_empty() =>
        {
            Some("scope matches no plan file: it can never measure a plan's proof budget".to_string())
        }
        // Nothing committed to compare the running catalog against, so
        // the rule passes every run while agreeing to nothing. `sf lock`
        // writes the fingerprint whenever this rule is enabled, so the
        // only way to reach this state is to delete the file or to enable
        // the rule without locking.
        CheckKind::CatalogTightening
            if !ctx.root.join(crate::fingerprint::CATALOG_LOCK_PATH).exists() =>
        {
            Some(format!(
                "no catalog fingerprint at {}: it has nothing to compare this binary's catalog against — run `sf lock`",
                crate::fingerprint::CATALOG_LOCK_PATH
            ))
        }
        _ => None,
    })
}

/// Why a `toolchain` rule can never produce a finding here, if any.
///
/// Split out of `inert_rules`'s match arm so this rule's own complexity
/// ceiling stays paid for by the branch that actually needs it, instead of
/// widening the meta-check's budget for every kind it covers.
fn inert_toolchain_reason(options: &Options, ctx: &Ctx) -> Option<String> {
    if options.tools.is_empty() {
        return Some("no tools declared: it can never find one missing".to_string());
    }
    // Non-empty, but no key names a language this project declares, so the
    // per-language loop in `checks::toolchain::run` `continue`s past every
    // one and the rule reports nothing, forever. Distinct from the empty-map
    // case above: that is a rule nobody configured, this is a rule configured
    // for a different project.
    if !covers_a_declared_language(options.tools.keys(), ctx) {
        return Some(format!(
            "tools declared for {}, none for {}: it will report zero findings for this project, forever, not merely \"no tool found\" — add a tool entry for the missing language, or disable this rule instance in policy and say why in docs/rules.md",
            joined(options.tools.keys()),
            joined(ctx.policy.project.languages.iter()),
        ));
    }
    None
}

/// Why an `evidence` rule can never produce a finding here, if any.
///
/// L3 is the layer the method calls "the whole method in one check", and it is
/// also the one that reaches inertness without anybody editing a rule: gates
/// live in their own top-level `gates:` map, so `enabled: true` and `gates: {}`
/// coexist perfectly happily and the rule reports nothing, forever. That state
/// is what this repository itself was in when the arm was written.
///
/// The second shape is the same hole as `inert_toolchain_reason`'s: a gate
/// exists, so the map is not empty, but its `activation` list is. `activated`
/// in `checks::evidence` matches a changed path against those globs and an
/// empty glob set matches nothing, so the gate can never turn on, under any
/// change, in either the known-change-set branch or the fail-closed one.
fn inert_evidence_reason(ctx: &Ctx) -> Option<String> {
    if ctx.policy.gates.is_empty() {
        return Some(
            "no gate declared in `gates:`: there is no customer-visible effect for it to require evidence of — declare one, or disable this rule in policy and say why in docs/rules.md".to_string(),
        );
    }
    let dead = gates_with_no_activation(ctx);
    if dead.len() == ctx.policy.gates.len() {
        return Some(format!(
            "every declared gate ({}) has an empty `activation`: no change to any file can ever turn one on, so this rule will report zero findings forever, not merely \"no evidence needed\"",
            joined(dead.iter()),
        ));
    }
    None
}

/// Why a `gate_coverage` rule can never produce a finding here, if any.
///
/// `gate_coverage` reads a gate's `plan` and compares the criteria there
/// against `required_assertions`. `plan` is an `Option`, and the loop skips a
/// gate without one, so a policy whose gates all omit it leaves the rule
/// enabled and silent — and silence here reads as "the gates cover their
/// plans", which is the opposite of what it means.
fn inert_gate_coverage_reason(ctx: &Ctx) -> Option<String> {
    if ctx.policy.gates.is_empty() {
        return Some(
            "no gate declared in `gates:`: there is no gate whose plan it could compare against — declare one, or disable this rule in policy and say why in docs/rules.md".to_string(),
        );
    }
    if ctx.policy.gates.values().all(|gate| gate.plan.is_none()) {
        return Some(format!(
            "no declared gate ({}) names a `plan`: this rule only reads gates that do, so it will report zero findings forever",
            joined(ctx.policy.gates.keys()),
        ));
    }
    None
}

/// The gates a change can never activate, because they select no path at all.
fn gates_with_no_activation(ctx: &Ctx) -> Vec<String> {
    ctx.policy
        .gates
        .iter()
        .filter(|(_, gate)| gate.activation.iter().all(|glob| glob.trim().is_empty()))
        .map(|(name, _)| name.clone())
        .collect()
}

#[cfg(test)]
mod inert_l3 {
    use super::super::Ctx;
    use crate::catalog::Catalog;
    use crate::policy::{FIXTURES_DIR, Policy, RULES_DIR};
    use crate::ratchet::Ratchet;
    use crate::scan;

    /// `L5.NO_INERT_RULE`'s own mutation fixture declares one gate with an
    /// empty `activation` and no `plan:`. Both L3 rules are switched on there
    /// and neither can ever fire: `checks::evidence::activated` matches
    /// against an empty glob set, and `gate_coverage` skips a gate with no
    /// plan. Deleting either arm from `inert_reason` above makes this test
    /// fail on the corresponding key.
    ///
    /// The gate is present rather than absent on purpose. `gates: {}` is the
    /// state this repository was actually in, and it is also the state a
    /// one-line `is_empty()` check would already catch — so the fixture holds
    /// the harder shape, exactly as the `tools:` map beside it names only a
    /// language nobody declares.
    #[test]
    fn a_gate_that_can_never_activate_makes_both_l3_rules_inert() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES_DIR)
            .join("L5.NO_INERT_RULE");
        let policy = Policy::load(&root).expect("the fixture policy loads");
        assert!(!policy.gates.is_empty(), "the fixture must declare a gate, not omit one");
        let mut catalog = Catalog::builtin().expect("the built-in catalog loads");
        catalog.extend_from_dir(&root.join(RULES_DIR)).expect("the fixture declares no local rules");
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
        };
        let rule = catalog.get("L5.NO_INERT_RULE").expect("ships in the catalog").clone();
        let findings = super::inert_rules(&rule, &ctx).expect("inert_rules runs");
        let message = |key: &str| {
            findings
                .iter()
                .find(|f| f.key == key)
                .unwrap_or_else(|| panic!("an inert L3 rule must be reported: {key}"))
                .message
                .clone()
        };

        let evidence = message("inert:L3.GATE_HAS_FRESH_EVIDENCE");
        assert!(evidence.contains("checkout"), "names the dead gate: {evidence}");
        assert!(
            evidence.contains("forever"),
            "says the rule can never fire here, not that no evidence is due: {evidence}"
        );

        let coverage = message("inert:L3.GATE_COVERS_THE_PLAN");
        assert!(coverage.contains("plan"), "names what the gate is missing: {coverage}");
        assert!(coverage.contains("forever"), "says the rule can never fire here: {coverage}");

        let budget = message("inert:L4.PLAN_PROOF_BUDGET@inert");
        assert!(budget.contains("scope"), "names the empty scope: {budget}");
        assert!(budget.contains("proof budget"), "names the rule's subject: {budget}");
    }
}

#[cfg(test)]
mod inert_forwarder {
    use crate::catalog::Catalog;
    use crate::checks::Ctx;
    use crate::policy::{FIXTURES_DIR, Policy};
    use crate::ratchet::Ratchet;
    use crate::scan;

    /// The forwarder rule ships a query for typescript, python and rust, and
    /// deliberately none for go or ruby. A repository that declares only go
    /// therefore enables a rule that can never fire, which is the state
    /// `L5.NO_INERT_RULE` exists to refuse. The policy is written here rather
    /// than taken from a fixture because every generated fixture declares all
    /// five languages, so none of them can hold this shape.
    const GO_ONLY: &str = "version: 1\nproject:\n  name: go-only\n  languages: [go]\nrules:\n  L1.INDIRECTION_EARNS_ITS_NAME:\n    enabled: true\n";

    #[test]
    fn a_forwarder_with_no_query_for_a_declared_language_is_inert() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES_DIR)
            .join("L5.NO_INERT_RULE");
        let policy: Policy = serde_yaml::from_str(GO_ONLY).expect("the policy parses");
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
        };
        let rule = catalog.get("L5.NO_INERT_RULE").expect("ships in the catalog").clone();
        let findings = super::inert_rules(&rule, &ctx).expect("inert_rules runs");
        let reported = findings
            .iter()
            .find(|f| f.key == "inert:L1.INDIRECTION_EARNS_ITS_NAME")
            .unwrap_or_else(|| panic!("the forwarder rule must be reported inert: {findings:?}"));
        assert!(
            reported.message.contains("no query for any language this repository declares"),
            "says why it can never fire: {}",
            reported.message
        );
    }
}

#[cfg(test)]
mod inert_toolchain {
    use crate::catalog::Catalog;
    use crate::checks::Ctx;
    use crate::policy::{FIXTURES_DIR, Policy, RULES_DIR};
    use crate::ratchet::Ratchet;
    use crate::scan;

    /// `L5.NO_INERT_RULE`'s own fixture enables
    /// `L6.DATA_RACES_ARE_DETECTED@toolchain_gap` with a `tools:` map naming
    /// only `java`, which its mini-repo never declares. Before `inert_rules`
    /// looked past `options.tools.is_empty()`, a non-empty map like this
    /// passed silently. Reverting the second `CheckKind::Toolchain` arm above
    /// makes this fail: no finding carries `inert:...@toolchain_gap`.
    #[test]
    fn a_non_empty_tools_map_missing_every_declared_language_is_inert() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES_DIR)
            .join("L5.NO_INERT_RULE");
        let policy = Policy::load(&root).expect("the fixture policy loads");
        let mut catalog = Catalog::builtin().expect("the built-in catalog loads");
        catalog
            .extend_from_dir(&root.join(RULES_DIR))
            .expect("the fixture declares no local rules");
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
        };
        let rule = catalog.get("L5.NO_INERT_RULE").expect("ships in the catalog").clone();
        let findings = super::inert_rules(&rule, &ctx).expect("inert_rules runs");
        let hit = findings
            .iter()
            .find(|f| f.key == "inert:L6.DATA_RACES_ARE_DETECTED@toolchain_gap")
            .expect(
                "a toolchain rule whose tools map names only a language nobody declares must be flagged inert",
            );
        assert!(hit.message.contains("java"), "names the tools the map does have: {}", hit.message);
        assert!(
            hit.message.contains("python"),
            "names the declared language it cannot cover: {}",
            hit.message
        );
        assert!(
            hit.message.contains("forever"),
            "says the rule would never fire here, not just that a tool is missing: {}",
            hit.message
        );
    }
}

/// Whether at least one file the rule's scope selects has an extension this
/// tool can parse into a language the project actually declares. Mirrors
/// `checks::complexity::run`'s own skip conditions exactly — both the
/// language-classification skip and the declared-language skip, since either
/// one alone leaves the ceiling silently inert.
fn any_parseable_declared_file(ctx: &Ctx, options: &Options) -> Result<bool> {
    let selected = scan::select(ctx.files, &options.scope, &options.exclude)?;
    Ok(selected.iter().any(|f| {
        Lang::from_path(&f.abs).is_some_and(|lang| {
            ctx.policy.project.languages.iter().any(|declared| declared == lang.name())
        })
    }))
}

/// Markdown link targets: `[text](target)` and reference definitions.
fn link_targets(body: &str) -> Result<Vec<String>> {
    let inline = Regex::new(r#"\]\(([^)\s]+)(?:\s+"[^"]*")?\)"#)?;
    let reference = Regex::new(r"(?m)^\[[^\]]+\]:\s*(\S+)")?;
    Ok(inline
        .captures_iter(body)
        .chain(reference.captures_iter(body))
        .map(|c| c[1].to_string())
        .collect())
}

/// Schemes that point somewhere other than this repository.
fn is_external(target: &str) -> bool {
    ["http://", "https://", "mailto:", "data:", "#"]
        .iter()
        .any(|prefix| target.starts_with(prefix))
}

fn doc_links(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Ok(body) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        let dir = file.abs.parent().unwrap_or(ctx.root);
        for target in link_targets(&body)? {
            if is_external(&target) {
                continue;
            }
            let bare = target.split('#').next().unwrap_or(&target);
            if bare.is_empty() {
                continue;
            }
            let resolved = if let Some(absolute) = bare.strip_prefix('/') {
                ctx.root.join(absolute)
            } else {
                dir.join(bare)
            };
            if !resolved.exists() {
                findings.push(
                    Finding::new(
                        &rule.id,
                        rule.severity,
                        file.rel.clone(),
                        format!("{}:{bare}", file.rel),
                        format!("link target `{bare}` does not exist"),
                    )
                    .actual(bare.to_string()),
                );
            }
        }
    }
    Ok(findings)
}

fn root_files(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let allowlist_name = opts.allowlist_file.as_deref().unwrap_or(".allowed-root-files");
    let allowlist_path = ctx.root.join(allowlist_name);
    let Ok(body) = std::fs::read_to_string(&allowlist_path) else {
        return Ok(vec![Finding::new(
            &rule.id,
            rule.severity,
            allowlist_name.to_string(),
            "missing-allowlist".to_string(),
            "this rule is enabled but the root allowlist does not exist",
        )
        .expected(format!("a {allowlist_name} listing every intended root file"))]);
    };
    let mut allowed: BTreeSet<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    allowed.insert(allowlist_name);

    // Both files and directories, derived from the walk so that gitignored
    // entries are absent: a permission gate seeded with .DS_Store is a gate
    // nobody trusts. A new root *directory* — `notes/`, `scratch/` — is the
    // same smell as a new root file and was previously invisible here.
    let mut entries: BTreeSet<String> = BTreeSet::new();
    for file in ctx.files {
        match file.rel.split_once('/') {
            Some((first, _)) => entries.insert(first.to_string()),
            None => entries.insert(file.rel.clone()),
        };
    }

    let mut findings = Vec::new();
    for name in entries {
        if allowed.contains(name.as_str()) {
            continue;
        }
        findings.push(
            Finding::new(
                &rule.id,
                rule.severity,
                name.clone(),
                name.clone(),
                format!("`{name}` is at the repository root but not declared"),
            )
            .expected(format!("an entry in {allowlist_name}, or somewhere with a lifecycle")),
        );
    }
    Ok(findings)
}

fn rule_citations(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let pattern = Regex::new(opts.marker.as_deref().unwrap_or(r"[A-Z][0-9]\.[A-Z_]+"))?;
    let mut cited: BTreeSet<String> = BTreeSet::new();
    let mut where_cited: Vec<(String, String)> = Vec::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Ok(body) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        for hit in pattern.find_iter(&body) {
            cited.insert(hit.as_str().to_string());
            where_cited.push((hit.as_str().to_string(), file.rel.clone()));
        }
    }

    let mut findings = Vec::new();
    for id in ctx.catalog.rules.keys().filter(|id| ctx.policy.any_instance_enabled(id)) {
        if !cited.contains(id) {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    opts.scope.join(", "),
                    format!("uncited:{id}"),
                    format!("{id} is enforced but never explained in prose"),
                )
                .expected(format!("a document citing {id} and the decision behind it")),
            );
        }
    }
    let templates = crate::interview::template_rule_ids();
    for (id, location) in &where_cited {
        if ctx.catalog.get(id).is_none() && !templates.contains(id) {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    location.clone(),
                    format!("unknown:{id}@{location}"),
                    format!("`{id}` is cited here but is not a rule in the catalog"),
                )
                .actual(id.clone()),
            );
        }
    }
    Ok(findings)
}

fn plan_cadence(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let order_rel = match opts.execution_order.as_deref() {
        Some(path) => path,
        None => return Ok(Vec::new()),
    };
    let order_body = std::fs::read_to_string(ctx.root.join(order_rel)).unwrap_or_default();
    let exit = Regex::new(
        opts.marker
            .as_deref()
            .unwrap_or(r"(?i)^[\s*_#>|-]*exit condition[\s*_]*[:|]"),
    )?;

    let mut findings = Vec::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        if file.rel == order_rel {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        if !body.lines().any(|line| exit.is_match(line)) {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    file.rel.clone(),
                    format!("no-exit:{}", file.rel),
                    "this plan never states what would make it finished",
                )
                .expected("an `Exit condition:` naming an externally visible effect"),
            );
        }
        let stem = std::path::Path::new(&file.rel)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if !order_body.contains(&stem) {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    file.rel.clone(),
                    format!("unordered:{}", file.rel),
                    format!("`{stem}` is not in the execution order"),
                )
                .expected(format!("a row in {order_rel}, or an explicit parked entry")),
            );
        }
    }
    Ok(findings)
}

/// `L5.EVERY_CHECK_HAS_A_MUTATION_TEST`: the fixture must exist. `sf verify`
/// is what proves it actually trips the rule.
fn mutation_coverage(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for id in ctx.catalog.rules.keys().filter(|id| ctx.policy.any_instance_enabled(id)) {
        let fixture = ctx.root.join(FIXTURES_DIR).join(id);
        if !fixture.is_dir() {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    format!("{FIXTURES_DIR}/{id}"),
                    format!("no-mutation:{id}"),
                    format!("{id} is enabled with nothing proving it ever fires"),
                )
                .expected(format!("a mutation fixture at {FIXTURES_DIR}/{id}/")),
            );
        }
    }
    Ok(findings)
}

/// The proof marker a criterion closes with, e.g.
/// `(proof: assertion:api.feedback_linked_to_acquisition)`.
///
/// Anchored at the end of the joined item on purpose: a marker in the middle of
/// a sentence would name a proof for a clause rather than for the criterion.
const CRITERION_MARKER: &str = r"\(proof:\s*([a-z_]+)\s*:\s*([^)]*)\)\s*$";

/// `assertion` and `test` name something that runs. `deferred` says the
/// criterion is not built and `unspecified` says no check has been designed for
/// it — both are legitimate to declare and both are debt, which is the point:
/// the admission becomes a line someone can grep instead of a sentence buried
/// in a long document.
const PROOF_KINDS: &[&str] = &["assertion", "test", "deferred", "unspecified"];

struct Criterion {
    line: usize,
    text: String,
    kind: Option<String>,
    value: String,
}

impl Criterion {
    /// A marker that parsed, named a known kind, and carried a value.
    fn is_complete(&self) -> bool {
        match &self.kind {
            Some(kind) => PROOF_KINDS.contains(&kind.as_str()) && !self.value.is_empty(),
            None => false,
        }
    }
}

/// Pull every checkbox criterion out of a plan, with the marker it closes with.
///
/// Criteria wrap, so the marker is looked for in the joined item rather than on
/// the checkbox line. A checkbox is the definition of a criterion because plans
/// spell the surrounding heading four different ways — "Acceptance criteria",
/// "Acceptance gates", "Gates", "Rollout/acceptance additions" — and a rule that
/// matches heading names is a rule about prose style.
fn parse_criteria(body: &str) -> Result<Vec<Criterion>> {
    let checkbox = Regex::new(r"^\s*-\s\[[ xX]\]\s*(.*)$")?;
    let continuation = Regex::new(r"^\s+\S")?;
    let marker = Regex::new(CRITERION_MARKER)?;

    let lines: Vec<&str> = body.lines().collect();
    let mut criteria = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(opened) = checkbox.captures(lines[index]) else {
            index += 1;
            continue;
        };
        let line = index + 1;
        let mut parts = vec![opened[1].trim().to_string()];
        index += 1;
        while index < lines.len()
            && continuation.is_match(lines[index])
            && !checkbox.is_match(lines[index])
        {
            parts.push(lines[index].trim().to_string());
            index += 1;
        }
        let joined = parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(" ");
        match marker.captures(&joined) {
            Some(found) => {
                let whole = found.get(0).map(|m| m.start()).unwrap_or(joined.len());
                criteria.push(Criterion {
                    line,
                    text: joined[..whole].trim().to_string(),
                    kind: Some(found[1].to_string()),
                    value: found[2].trim().to_string(),
                });
            }
            None => criteria.push(Criterion { line, text: joined, kind: None, value: String::new() }),
        }
    }
    Ok(criteria)
}

/// `L4.PLAN_CRITERION_NAMES_ITS_CHECK`: a criterion with nothing that proves it.
///
/// A plan states criteria in prose and the gate enforces a list of assertions.
/// Where nothing joins the two, both can be honest and the pair still says
/// nothing: the criterion is promised, the gate never covered it, and the only
/// place that records the gap is the plan nobody re-reads.
fn plan_criteria(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Ok(body) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        for criterion in parse_criteria(&body)? {
            if criterion.is_complete() {
                continue;
            }
            let detail = match &criterion.kind {
                None => "names no check that would prove it".to_string(),
                Some(kind) if !PROOF_KINDS.contains(&kind.as_str()) => {
                    format!("names unknown proof kind `{kind}`")
                }
                Some(kind) => format!("carries a `{kind}` marker with no value"),
            };
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    format!("{}:{}", file.rel, criterion.line),
                    format!("unproven:{}:{}", file.rel, criterion.line),
                    format!("this acceptance criterion {detail}"),
                )
                .expected(
                    "a trailing (proof: assertion:ID | test:PATH | deferred:REASON \
                     | unspecified:REASON)",
                )
                .actual(criterion.text),
            );
        }
    }
    Ok(findings)
}

/// `L4.PLAN_PROOF_BUDGET`: a plan cannot erase its promises to evade a debt
/// ceiling, and it cannot carry more undeclared proof debt than its budget.
fn plan_proof_budget(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let max = opts.max.ok_or_else(|| anyhow::anyhow!("{} needs a `max` percentage", rule.id))?;
    let mut findings = Vec::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Ok(body) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        findings.extend(plan_proof_budget_findings(rule, &file.rel, &body, max)?);
    }
    Ok(findings)
}

fn plan_proof_budget_findings(
    rule: &Rule,
    path: &str,
    body: &str,
    max: usize,
) -> Result<Vec<Finding>> {
    let criteria = parse_criteria(body)?;
    if criteria.is_empty() {
        return Ok(vec![
            Finding::new(
                &rule.id,
                rule.severity,
                path,
                format!("no-criteria:{path}"),
                "this plan has no acceptance criteria",
            )
            .expected("at least one acceptance criterion with a proof marker")
            .actual("none"),
        ]);
    }

    let debt = criteria
        .iter()
        .filter(|criterion| matches!(criterion.kind.as_deref(), Some("deferred" | "unspecified")))
        .count();
    if debt * 100 <= max * criteria.len() {
        return Ok(Vec::new());
    }
    let percentage = debt * 100 / criteria.len();
    Ok(vec![
        Finding::new(
            &rule.id,
            rule.severity,
            path,
            format!("debt-budget:{path}"),
            format!(
                "this plan carries {debt} deferred or unspecified criterion(s) out of {} ({percentage}%), above its {max}% proof budget",
                criteria.len()
            ),
        )
        .expected(format!("at most {max}% deferred or unspecified criteria"))
        .actual(format!("{percentage}%")),
    ])
}

#[cfg(test)]
mod proof_budget {
    use super::plan_proof_budget_findings;
    use crate::catalog::Catalog;

    fn rule() -> crate::catalog::Rule {
        Catalog::builtin()
            .expect("the shipped catalog loads")
            .get("L4.PLAN_PROOF_BUDGET")
            .expect("the rule ships")
            .clone()
    }

    #[test]
    fn the_floor_and_ceiling_are_independent() {
        let rule = rule();
        let floor = plan_proof_budget_findings(&rule, "plans/floor.md", "# Floor\n", 60)
            .expect("the floor plan parses");
        assert_eq!(floor[0].key, "no-criteria:plans/floor.md");

        let ceiling = plan_proof_budget_findings(
            &rule,
            "plans/ceiling.md",
            "- [ ] First debt. (proof: deferred:not designed)\n- [ ] Second debt. (proof: unspecified:not designed)\n- [ ] A proof. (proof: test:tests/proof.rs)\n",
            60,
        )
        .expect("the ceiling plan parses");
        assert_eq!(ceiling[0].key, "debt-budget:plans/ceiling.md");

        let within_budget = plan_proof_budget_findings(
            &rule,
            "plans/within-budget.md",
            "- [ ] Debt. (proof: deferred:not designed)\n- [ ] A proof. (proof: test:tests/proof.rs)\n",
            60,
        )
        .expect("the control plan parses");
        assert!(within_budget.is_empty(), "a plan under the budget must stay quiet");
    }
}

/// `L3.GATE_COVERS_THE_PLAN`: the plan names a proof the gate never asks for,
/// or names no non-deferred criterion while the gate requires nothing.
///
/// This is the half `L3.GATE_HAS_FRESH_EVIDENCE` cannot see. That rule verifies
/// the evidence for what the gate demanded; it has no way to know the gate
/// demanded less than the plan promised. An assertion no run is required to
/// carry reads exactly like coverage.
fn gate_coverage(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (name, gate) in &ctx.policy.gates {
        let Some(plan) = &gate.plan else {
            continue;
        };
        let path = ctx.root.join(plan);
        let Ok(body) = std::fs::read_to_string(&path) else {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    plan.clone(),
                    format!("missing-plan:{name}"),
                    format!("gate `{name}` names a plan that does not exist"),
                )
                .expected(plan.clone()),
            );
            continue;
        };
        let criteria = parse_criteria(&body)?;
        if gate.required_assertions.is_empty()
            && !criteria
                .iter()
                .any(|criterion| matches!(criterion.kind.as_deref(), Some("assertion" | "test")))
        {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    plan.clone(),
                    format!("empty-requirements:{name}"),
                    format!(
                        "gate `{name}` requires no assertions and its plan names no undeferred criterion"
                    ),
                )
                .expected("a required assertion, or a plan criterion with an assertion or test proof")
                .actual("no required assertions; only deferred, unspecified, or no criteria"),
            );
        }
        for criterion in criteria {
            if criterion.kind.as_deref() != Some("assertion") {
                continue;
            }
            if gate.required_assertions.contains(&criterion.value) {
                continue;
            }
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    format!("{plan}:{}", criterion.line),
                    format!("uncovered:{name}:{}", criterion.value),
                    format!(
                        "this criterion is proven by `{}`, which gate `{name}` does not require",
                        criterion.value
                    ),
                )
                .expected(format!("`{}` in gates.{name}.required_assertions", criterion.value))
                .actual(if gate.required_assertions.is_empty() {
                    "the gate requires no assertions at all".to_string()
                } else {
                    gate.required_assertions.join(", ")
                }),
            );
        }
    }
    Ok(findings)
}

/// An `sf` invocation quoted in prose, e.g. `sf seal <gate>`. Only spans that
/// *open* with the tool's name are invocations; a backtick span that merely
/// mentions it somewhere in the middle is a sentence.
const CITED_COMMAND: &str = r"`sf ([^`]+)`";

/// `L4.RULE_PROSE_NAMES_A_REAL_COMMAND`: prose telling the reader to run
/// something this binary does not have.
///
/// The accepted surface comes from the command-line definition itself, so the
/// check cannot drift from the tool the way the prose just did.
fn rule_commands(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    let accepted = crate::accepted_commands();
    let cited = Regex::new(CITED_COMMAND)?;
    let document = ctx.policy.docs.rules_document().to_string();
    let mut findings = Vec::new();
    for (id, candidate) in &ctx.catalog.rules {
        if !ctx.policy.any_instance_enabled(id) {
            continue;
        }
        let prose = [
            ("statement", &candidate.statement),
            ("why", &candidate.why),
            ("fix", &candidate.fix),
        ];
        for (field, text) in prose {
            for hit in cited.captures_iter(text) {
                let invocation = hit[1].trim();
                let Some((problem, expected)) = unaccepted(invocation, &accepted) else {
                    continue;
                };
                findings.push(
                    Finding::new(
                        &rule.id,
                        rule.severity,
                        document.clone(),
                        format!("dead-command:{id}:{invocation}"),
                        format!("{id}'s `{field}` says to run `sf {invocation}`, and {problem}"),
                    )
                    .expected(expected)
                    .actual(format!("sf {invocation}")),
                );
            }
        }
    }
    Ok(findings)
}

/// What is wrong with an invocation, and what this binary would have accepted.
/// `None` means it would run.
fn unaccepted(
    invocation: &str,
    accepted: &BTreeMap<String, BTreeSet<String>>,
) -> Option<(String, String)> {
    let mut tokens = invocation.split_whitespace();
    let name = tokens.next()?;
    // Nothing here names a subcommand, so there is nothing to be wrong about.
    // Either prose is describing the *shape* of an invocation (`sf <command>`,
    // `sf ...`) rather than telling anyone to run it, or a global flag came
    // first — and deciding which of the tokens after `--root` is its value
    // would invent findings. Rule prose does not write invocations that way.
    if name.starts_with('-') || name.starts_with('<') || name.chars().all(|c| c == '.') {
        return None;
    }
    let Some(flags) = accepted.get(name) else {
        return Some((
            format!("`{name}` is not a subcommand this `sf` has"),
            format!("one of: {}", joined(accepted.keys())),
        ));
    };
    // Long flags only. Everything else in an invocation is a value or a
    // placeholder — `<gate>`, `L1.COMPLEXITY_CEILING`, `origin/main` — and a
    // rule that guessed at those would produce findings nobody believes.
    for token in tokens {
        let flag = token.split('=').next().unwrap_or(token);
        if !flag.starts_with("--") || flags.contains(flag) {
            continue;
        }
        return Some((
            format!("`sf {name}` does not accept `{flag}`"),
            format!("`sf {name}` with one of: {}", joined(flags.iter())),
        ));
    }
    None
}

fn joined<'a>(names: impl Iterator<Item = &'a String>) -> String {
    names.cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod cited_commands {
    use super::unaccepted;
    use crate::catalog::Catalog;

    /// Every invocation the shipped catalog quotes, including the rules this
    /// repository has switched off. The check itself only reads enabled rules,
    /// because a repository is not answerable for prose it never shows anyone;
    /// the catalog is ours, and a dead command in it ships to everybody.
    #[test]
    fn the_shipped_catalog_quotes_only_real_commands() {
        let accepted = crate::accepted_commands();
        let cited = regex::Regex::new(super::CITED_COMMAND).expect("the pattern compiles");
        let catalog = Catalog::builtin().expect("the built-in catalog loads");
        let mut dead = Vec::new();
        for rule in catalog.rules.values() {
            for text in [&rule.statement, &rule.why, &rule.fix] {
                for hit in cited.captures_iter(text) {
                    let invocation = hit[1].trim();
                    if let Some((problem, _)) = unaccepted(invocation, &accepted) {
                        dead.push(format!("{}: `sf {invocation}` — {problem}", rule.id));
                    }
                }
            }
        }
        assert!(dead.is_empty(), "catalog prose names commands sf does not accept:\n{dead:#?}");
    }

    #[test]
    fn a_real_invocation_is_accepted() {
        let accepted = crate::accepted_commands();
        for invocation in [
            "verify",
            "seal <gate>",
            "explain L1.COMPLEXITY_CEILING",
            "ratchet --months 6",
            "check --changed origin/main",
            "check --format=json",
            "init --language typescript --layer L1,L4,L5",
            // Prose about the form of an invocation, not an invocation.
            "...",
            "<subcommand> --help",
        ] {
            assert_eq!(unaccepted(invocation, &accepted), None, "{invocation}");
        }
    }

    #[test]
    fn a_subcommand_that_does_not_exist_is_a_finding() {
        let accepted = crate::accepted_commands();
        let (problem, expected) =
            unaccepted("evidence record", &accepted).expect("`sf evidence` is not a command");
        assert!(problem.contains("evidence"), "{problem}");
        assert!(expected.contains("seal"), "the expectation lists the real commands: {expected}");
    }

    /// The narrower half, and the one that was live in this catalog: a real
    /// subcommand carrying a flag it never had.
    #[test]
    fn a_flag_the_subcommand_does_not_take_is_a_finding() {
        let accepted = crate::accepted_commands();
        let (problem, _) =
            unaccepted("lock --update", &accepted).expect("`sf lock` takes no --update");
        assert!(problem.contains("--update"), "{problem}");
    }
}

/// The marker a page puts above a promise. An HTML comment rather than syntax,
/// because it has to work in markdown, MDX and HTML with no build step and
/// stay invisible in the rendered page.
const CLAIM_MARKER: &str = r"<!--\s*claim:(.*?)-->";

/// The half of the marker that names the gate.
const PROVEN_BY: &str = r"proven-by:\s*(\S+)";

/// A parsed marker: the promise's stable id, and the gate it says proves it.
struct Claim {
    id: Option<String>,
    gate: Option<String>,
}

/// `L4.CLAIM_CITES_ITS_EVIDENCE`: a promise on a page, joined to the gate that
/// proved it.
///
/// Two directions, not three. Every claim names a gate and every named gate
/// exists; "every gate is claimed somewhere" is the wrong third, because most
/// gates have nothing to do with a page anyone reads. Freshness is not checked
/// here either: a named gate carries evidence, and `L3.GATE_HAS_FRESH_EVIDENCE`
/// already fails once the implementation digest moves, so the promise goes red
/// *through* the gate rather than through a worse copy of that logic.
fn claim_citations(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let marker = Regex::new(opts.marker.as_deref().unwrap_or(CLAIM_MARKER))?;
    let proven_by = Regex::new(PROVEN_BY)?;
    let mut findings = Vec::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Ok(body) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        let scanned = without_fences(&body);
        for hit in marker.captures_iter(&scanned) {
            let at = hit.get(0).map(|m| m.start()).unwrap_or(0);
            let line = scanned[..at].matches('\n').count() + 1;
            let inside = hit.get(1).map(|m| m.as_str()).unwrap_or("");
            let claim = parse_claim(inside, &proven_by);
            let Some((problem, expected)) = unproven(&claim, ctx) else {
                continue;
            };
            let id = claim.id.clone().unwrap_or_else(|| format!("unnamed-{line}"));
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    format!("{}:{line}", file.rel),
                    format!("claim:{}:{id}", file.rel),
                    problem,
                )
                .expected(expected)
                .actual(inside.trim().to_string()),
            );
        }
    }
    Ok(findings)
}

fn parse_claim(inside: &str, proven_by: &Regex) -> Claim {
    Claim {
        id: inside
            .split_whitespace()
            .next()
            .filter(|token| !token.starts_with("proven-by"))
            .map(str::to_string),
        gate: proven_by.captures(inside).map(|found| found[1].to_string()),
    }
}

/// What is wrong with a claim, and what the marker should have said. `None`
/// means the promise is joined to a gate this policy declares.
fn unproven(claim: &Claim, ctx: &Ctx) -> Option<(String, String)> {
    let declared = || {
        if ctx.policy.gates.is_empty() {
            "this policy declares no gates at all".to_string()
        } else {
            format!("one of: {}", joined(ctx.policy.gates.keys()))
        }
    };
    let Some(id) = &claim.id else {
        return Some((
            "a claim marker here carries no id, so the promise cannot survive an edit of \
             the sentence around it"
                .to_string(),
            "a claim id, then the gate that proves it".to_string(),
        ));
    };
    let Some(gate) = &claim.gate else {
        return Some((
            format!("the claim `{id}` names nothing that proves it"),
            format!("a `proven-by:` naming a gate, {}", declared()),
        ));
    };
    if ctx.policy.gates.contains_key(gate) {
        return None;
    }
    Some((
        format!("the claim `{id}` is proven by `{gate}`, which the policy does not declare"),
        declared(),
    ))
}

/// Blank out fenced code, keeping every byte offset and newline so line
/// numbers still hold. A marker inside a fence is a page showing the syntax,
/// not a page making the promise — and the prose documenting this rule has to
/// be able to show it without tripping it.
fn without_fences(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut fenced = false;
    for line in body.split_inclusive('\n') {
        let fence = line.trim_start().starts_with("```");
        if fence || fenced {
            out.extend(line.bytes().map(|byte| if byte == b'\n' { '\n' } else { ' ' }));
        } else {
            out.push_str(line);
        }
        if fence {
            fenced = !fenced;
        }
    }
    out
}

#[cfg(test)]
mod claims {
    use super::{parse_claim, without_fences, PROVEN_BY};
    use regex::Regex;

    fn parsed(inside: &str) -> (Option<String>, Option<String>) {
        let claim = parse_claim(inside, &Regex::new(PROVEN_BY).expect("the pattern compiles"));
        (claim.id, claim.gate)
    }

    #[test]
    fn a_complete_marker_carries_an_id_and_a_gate() {
        let (id, gate) = parsed(" IMPORT_50K_UNDER_60S proven-by: bulk-import ");
        assert_eq!(id.as_deref(), Some("IMPORT_50K_UNDER_60S"));
        assert_eq!(gate.as_deref(), Some("bulk-import"));
    }

    #[test]
    fn a_marker_with_no_proof_keeps_its_id() {
        let (id, gate) = parsed(" SEARCH_IS_INSTANT ");
        assert_eq!(id.as_deref(), Some("SEARCH_IS_INSTANT"));
        assert_eq!(gate, None);
    }

    /// Without this the id would be read as `proven-by:` and the marker would
    /// look complete while naming nothing that survives an edit.
    #[test]
    fn a_marker_with_no_id_reports_no_id() {
        let (id, gate) = parsed(" proven-by: bulk-import ");
        assert_eq!(id, None);
        assert_eq!(gate.as_deref(), Some("bulk-import"));
    }

    #[test]
    fn a_fenced_marker_is_documentation_and_line_numbers_survive() {
        let body = "# Page\n\n```\n<!-- claim: EXAMPLE proven-by: nothing -->\n```\n\n<!-- claim: REAL proven-by: gate -->\n";
        let scanned = without_fences(body);
        assert_eq!(scanned.len(), body.len(), "byte offsets have to hold");
        assert_eq!(scanned.matches("<!-- claim:").count(), 1, "only the unfenced one survives");
        assert!(scanned.contains("REAL"), "{scanned}");
    }
}
