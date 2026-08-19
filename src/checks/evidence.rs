//! L3 — the effect gate.
//!
//! The one check that carries the whole method:
//!
//! * activation comes from touched paths, so no label or pull-request
//!   sentence can route around it;
//! * the manifest is re-verified, not trusted, so a summary cannot assert a
//!   pass the raw report never contained;
//! * the implementation digest is recorded, so evidence expires the moment
//!   the code it certified changes.

use super::Ctx;
use crate::catalog::Rule;
use crate::digest;
use crate::finding::{Finding, Severity};
use crate::policy::{Gate, Options};
use crate::scan;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Run {
    pub scenario: String,
    pub status: String,
    /// What performed the run. A human name is fine; "scripted" is not a proof.
    pub actor: String,
    pub report: String,
    #[serde(default)]
    pub report_sha256: String,
    #[serde(default)]
    pub required_assertions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub gate: String,
    /// Digest of the activation paths at the time the evidence was sealed.
    #[serde(default)]
    pub implementation_sha256: String,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, Deserialize)]
struct Report {
    #[serde(default)]
    scenario: String,
    #[serde(default)]
    status: String,
    /// What the actor was asked to do. Checked for leaked answers.
    #[serde(default)]
    goal: String,
    #[serde(default)]
    assertions: Vec<AssertionResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct AssertionResult {
    #[serde(rename = "type")]
    kind: String,
    status: String,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

/// Digest every file under the gate's activation paths, as a set.
pub fn implementation_digest(gate: &Gate, ctx: &Ctx) -> Result<String> {
    let mut entries = Vec::new();
    for file in scan::select(ctx.files, &gate.activation, &[])? {
        entries.push((file.rel.clone(), digest::file(&file.abs)?));
    }
    Ok(digest::tree(&mut entries))
}

fn activated(gate: &Gate, ctx: &Ctx) -> Result<bool> {
    let activation = scan::globs(&gate.activation)?;
    match &ctx.changed {
        // A known change set: activate when the work touched the gate's paths.
        Some(changed) => Ok(changed.iter().any(|path| activation.is_match(path))),
        // No change set: activate whenever the implementation exists at all.
        // Fail-closed — "we could not tell" must never resolve to "skip".
        None => Ok(ctx.files.iter().any(|f| activation.is_match(&f.rel))),
    }
}

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (name, gate) in &ctx.policy.gates {
        if !activated(gate, ctx)? {
            continue;
        }
        findings.extend(check_gate(rule, opts, ctx, name, gate)?);
    }
    Ok(findings)
}

fn fail(rule: &Rule, location: impl Into<String>, key: String, message: impl Into<String>) -> Finding {
    Finding::new(&rule.id, Severity::Critical, location, key, message)
}

fn check_gate(
    rule: &Rule,
    opts: &Options,
    ctx: &Ctx,
    name: &str,
    gate: &Gate,
) -> Result<Vec<Finding>> {
    let manifest_path = ctx.root.join(&gate.evidence);
    if !manifest_path.exists() {
        return Ok(vec![
            fail(
                rule,
                gate.evidence.clone(),
                format!("{name}:missing"),
                format!("gate `{name}` is active and has no evidence"),
            )
            .expected("a sealed evidence manifest")
            .actual("missing"),
        ]);
    }
    let manifest = match Manifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            return Ok(vec![fail(
                rule,
                gate.evidence.clone(),
                format!("{name}:unreadable"),
                format!("evidence for `{name}` cannot be read: {e}"),
            )]);
        }
    };

    let mut findings = check_manifest_identity(rule, ctx, name, gate, &manifest)?;
    for run in &manifest.runs {
        findings.extend(check_run(rule, opts, ctx, name, gate, run)?);
    }
    Ok(findings)
}

/// Identity, staleness and emptiness — the properties of the manifest itself.
fn check_manifest_identity(
    rule: &Rule,
    ctx: &Ctx,
    name: &str,
    gate: &Gate,
    manifest: &Manifest,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    if manifest.schema_version != 1 || manifest.gate != name {
        findings.push(
            fail(
                rule,
                gate.evidence.clone(),
                format!("{name}:identity"),
                "evidence identity does not match the active gate",
            )
            .expected(format!("schema_version 1, gate {name}"))
            .actual(format!("schema_version {}, gate {}", manifest.schema_version, manifest.gate)),
        );
    }
    // The check that stops evidence from certifying code it never saw.
    let current = implementation_digest(gate, ctx)?;
    if manifest.implementation_sha256 != current {
        findings.push(
            fail(
                rule,
                gate.evidence.clone(),
                format!("{name}:stale"),
                format!("the implementation changed since gate `{name}` was proven"),
            )
            .expected(current)
            .actual(if manifest.implementation_sha256.is_empty() {
                "never sealed".to_string()
            } else {
                manifest.implementation_sha256.clone()
            }),
        );
    }
    if manifest.runs.is_empty() {
        findings.push(fail(
            rule,
            gate.evidence.clone(),
            format!("{name}:no-runs"),
            format!("gate `{name}` has an evidence manifest with no runs in it"),
        ));
    }
    Ok(findings)
}

/// Resolve a run's report and re-verify its digest. `Err` on a fatal finding
/// that makes the rest of the run unjudgeable.
fn resolve_report(
    rule: &Rule,
    ctx: &Ctx,
    key: &str,
    run: &Run,
) -> Result<std::result::Result<Report, Finding>> {
    // Evidence that points outside the repository is evidence nobody else can
    // re-verify.
    let Ok(canonical) = ctx.root.join(&run.report).canonicalize() else {
        return Ok(Err(fail(
            rule,
            run.report.clone(),
            format!("{key}:report-missing"),
            format!("the report for `{}` is missing", run.scenario),
        )
        .actual(run.report.clone())));
    };
    if !canonical.starts_with(ctx.root.canonicalize()?) {
        return Ok(Err(fail(
            rule,
            run.report.clone(),
            format!("{key}:report-escapes"),
            format!("the report for `{}` resolves outside the repository", run.scenario),
        )));
    }
    let actual_digest = digest::file(&canonical)?;
    if run.report_sha256 != actual_digest {
        return Ok(Err(fail(
            rule,
            run.report.clone(),
            format!("{key}:report-digest"),
            format!("the report for `{}` does not match its recorded digest", run.scenario),
        )
        .expected(actual_digest)
        .actual(run.report_sha256.clone())));
    }
    match serde_json::from_str(&std::fs::read_to_string(&canonical)?) {
        Ok(report) => Ok(Ok(report)),
        Err(e) => Ok(Err(fail(
            rule,
            run.report.clone(),
            format!("{key}:report-invalid"),
            format!("the report for `{}` is not a valid report: {e}", run.scenario),
        ))),
    }
}

fn check_run(
    rule: &Rule,
    opts: &Options,
    ctx: &Ctx,
    name: &str,
    gate: &Gate,
    run: &Run,
) -> Result<Vec<Finding>> {
    let key = format!("{name}:{}", run.scenario);
    if run.status != "passed" {
        return Ok(vec![
            fail(
                rule,
                gate.evidence.clone(),
                format!("{key}:status"),
                format!("run `{}` is not a pass", run.scenario),
            )
            .expected("passed")
            .actual(run.status.clone()),
        ]);
    }
    let report = match resolve_report(rule, ctx, &key, run)? {
        Ok(report) => report,
        Err(finding) => return Ok(vec![finding]),
    };
    if report.status != "passed" || (!report.scenario.is_empty() && report.scenario != run.scenario)
    {
        return Ok(vec![
            fail(
                rule,
                run.report.clone(),
                format!("{key}:report-contradicts"),
                format!(
                    "the raw report for `{}` does not show a pass of that scenario",
                    run.scenario
                ),
            )
            .expected(format!("scenario {} status passed", run.scenario))
            .actual(format!("scenario {} status {}", report.scenario, report.status)),
        ]);
    }
    let mut findings = check_assertions(rule, &key, gate, run, &report);
    findings.extend(check_goal_fidelity(rule, opts, &key, run, &report));
    Ok(findings)
}

/// The union of what policy demands and what the manifest admits it owed.
///
/// A manifest is written by the change under review, so a list that lives only
/// there is a run declaring its own obligations — under-declare, and the gate
/// passes on a subset of what it was for. Policy sits outside the candidate
/// implementation, so unioning the two means a manifest can add an assertion
/// but never drop one.
fn required_assertions(gate: &Gate, run: &Run) -> Vec<String> {
    let mut all: Vec<String> = gate.required_assertions.clone();
    for assertion in &run.required_assertions {
        if !all.contains(assertion) {
            all.push(assertion.clone());
        }
    }
    all
}

fn check_assertions(
    rule: &Rule,
    key: &str,
    gate: &Gate,
    run: &Run,
    report: &Report,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for assertion in &required_assertions(gate, run) {
        match report.assertions.iter().find(|a| &a.kind == assertion) {
            None => findings.push(
                fail(
                    rule,
                    run.report.clone(),
                    format!("{key}:missing-assertion:{assertion}"),
                    format!("required assertion `{assertion}` is absent from the report"),
                )
                .expected(assertion.clone()),
            ),
            Some(result) if result.status != "passed" => findings.push(
                fail(
                    rule,
                    run.report.clone(),
                    format!("{key}:assertion:{assertion}"),
                    format!("required assertion `{assertion}` did not pass"),
                )
                .expected("passed")
                .actual(result.status.clone()),
            ),
            Some(_) => {}
        }
    }
    // An assertion the harness could not evaluate is not a pass. Counting one
    // as a pass is the most common way a green gate proves nothing.
    for result in report.assertions.iter().filter(|a| a.status == "unsupported") {
        findings.push(fail(
            rule,
            run.report.clone(),
            format!("{key}:unsupported:{}", result.kind),
            format!("assertion `{}` was unsupported, which is not a pass", result.kind),
        ));
    }
    findings
}

/// A goal that names the source tree is a replay recipe, not a customer.
fn check_goal_fidelity(
    rule: &Rule,
    opts: &Options,
    key: &str,
    run: &Run,
    report: &Report,
) -> Vec<Finding> {
    opts.forbidden_in_goal
        .iter()
        .filter(|forbidden| report.goal.contains(forbidden.as_str()))
        .map(|forbidden| {
            fail(
                rule,
                run.report.clone(),
                format!("{key}:goal:{forbidden}"),
                format!("the goal for `{}` hands the actor `{forbidden}`", run.scenario),
            )
            .expected("a goal phrased the way a customer would phrase it")
            .actual(forbidden.clone())
        })
        .collect()
}

/// Recompute every digest in a manifest from what is on disk.
pub fn seal(root: &Path, gate_name: &str, gate: &Gate, ctx: &Ctx) -> Result<Manifest> {
    let path = root.join(&gate.evidence);
    let mut manifest = Manifest::load(&path)?;
    manifest.schema_version = 1;
    manifest.gate = gate_name.to_string();
    manifest.implementation_sha256 = implementation_digest(gate, ctx)?;
    for run in &mut manifest.runs {
        let report = root.join(&run.report);
        anyhow::ensure!(report.exists(), "report {} does not exist", run.report);
        run.report_sha256 = digest::file(&report)?;
    }
    std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    Ok(manifest)
}
