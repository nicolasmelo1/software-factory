//! L4 — how documentation, plans and rules stay attached to each other.
//!
//! These are the cheapest checks in the tool and the ones a greenfield repo
//! should turn on first: they cost three markdown files and they are what
//! keeps the other layers from drifting into folklore.

use super::Ctx;
use crate::catalog::{CadenceMode, CheckKind, Rule};
use crate::finding::Finding;
use crate::policy::{FIXTURES_DIR, Options};
use crate::scan;
use anyhow::Result;
use regex::Regex;
use std::collections::BTreeSet;

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx, mode: CadenceMode) -> Result<Vec<Finding>> {
    match mode {
        CadenceMode::DocLinks => doc_links(rule, opts, ctx),
        CadenceMode::RootFiles => root_files(rule, opts, ctx),
        CadenceMode::RuleCitations => rule_citations(rule, opts, ctx),
        CadenceMode::PlanCadence => plan_cadence(rule, opts, ctx),
        CadenceMode::PlanCriteria => plan_criteria(rule, opts, ctx),
        CadenceMode::GateCoverage => gate_coverage(rule, ctx),
        CadenceMode::MutationCoverage => mutation_coverage(rule, ctx),
        CadenceMode::InertRules => inert_rules(rule, ctx),
    }
}

fn covers_a_declared_language<'a>(
    mut languages: impl Iterator<Item = &'a String>,
    ctx: &Ctx,
) -> bool {
    languages.any(|name| ctx.policy.project.languages.contains(name))
}

/// An enabled rule pointed at nothing. It passes every run and reads exactly
/// like a rule that is protecting you.
fn inert_rules(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (instance, base) in ctx.policy.instances() {
        let Some(candidate) = ctx.catalog.get(&base) else {
            continue;
        };
        let (id, candidate) = (&instance, &super::as_instance(candidate, &instance));
        let options = super::options_for(candidate, ctx.policy)?;
        let reason = match &candidate.check {
            CheckKind::Lock if options.scope.is_empty() => "no scope: it locks nothing",
            CheckKind::Command if options.run.is_none() => {
                "no command set: there is nothing for it to run"
            }
            CheckKind::Toolchain if options.tools.is_empty() => {
                "no tools declared: it can never find one missing"
            }
            CheckKind::Shape { languages }
                if !covers_a_declared_language(languages.keys(), ctx) =>
            {
                "no query for any language this repository declares"
            }
            CheckKind::Nested { languages }
                if !covers_a_declared_language(languages.keys(), ctx) =>
            {
                "no query for any language this repository declares"
            }
            _ => continue,
        };
        findings.push(
            Finding::new(
                &rule.id,
                rule.severity,
                crate::policy::POLICY_PATH,
                format!("inert:{id}"),
                format!("{id} is enabled but cannot produce a finding — {reason}"),
            )
            .expected("a configured rule, or an honest `enabled: false`"),
        );
    }
    Ok(findings)
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

/// `L3.GATE_COVERS_THE_PLAN`: the plan names a proof the gate never asks for.
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
        for criterion in parse_criteria(&body)? {
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
