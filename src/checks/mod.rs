//! Check engines. Each catalog rule names one; the rule's options are data.

pub mod cadence;
pub mod complexity;
pub mod evidence;
pub mod lock;
pub mod shape;
pub mod text_pattern;

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
    pub today: String,
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

/// Run every enabled rule. Returns findings in stable rule order.
pub fn run_all(ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (id, rule) in &ctx.catalog.rules {
        if ctx.policy.enabled(id).is_none() {
            continue;
        }
        findings.extend(run_one(rule, ctx)?);
    }
    Ok(findings)
}

pub fn run_one(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    let opts = options_for(rule, ctx.policy)?;
    match &rule.check {
        CheckKind::Shape { languages } => shape::run(rule, &opts, languages, ctx),
        CheckKind::Complexity => complexity::run(rule, &opts, ctx),
        CheckKind::TextPattern => text_pattern::run(rule, &opts, ctx),
        CheckKind::Lock => lock::run(rule, &opts, ctx),
        CheckKind::Expiry => lock::expiry(rule, ctx),
        CheckKind::Cadence { mode } => cadence::run(rule, &opts, ctx, *mode),
        CheckKind::Evidence => evidence::run(rule, &opts, ctx),
    }
}
