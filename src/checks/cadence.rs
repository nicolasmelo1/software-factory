//! L4 — how documentation, plans and rules stay attached to each other.
//!
//! These are the cheapest checks in the tool and the ones a greenfield repo
//! should turn on first: they cost three markdown files and they are what
//! keeps the other layers from drifting into folklore.

use super::Ctx;
use crate::catalog::{CadenceMode, Rule};
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
        CadenceMode::MutationCoverage => mutation_coverage(rule, ctx),
    }
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

    let mut findings = Vec::new();
    for entry in std::fs::read_dir(ctx.root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
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
            .expected(format!("an entry in {allowlist_name}, or the file somewhere with a lifecycle")),
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
    for (id, _) in ctx.catalog.rules.iter().filter(|(id, _)| ctx.policy.enabled(id).is_some()) {
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
    for (id, location) in &where_cited {
        if ctx.catalog.get(id).is_none() {
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
    for (id, _) in ctx.catalog.rules.iter().filter(|(id, _)| ctx.policy.enabled(id).is_some()) {
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
