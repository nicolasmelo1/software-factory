//! Check engines. Each catalog rule names one; the rule's options are data.

pub mod cadence;
pub mod command;
pub mod complexity;
pub mod evidence;
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
        findings.extend(run_one(&as_instance(rule, &instance), ctx)?);
    }
    Ok(findings)
}

/// The same rule under an instance's name, so its findings, its options and
/// its ratchet entries stay separate from the other instances'.
pub fn as_instance(rule: &Rule, instance: &str) -> Rule {
    let mut copy = rule.clone();
    copy.id = instance.to_string();
    copy
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
        CheckKind::Nested { languages } => nested::run(rule, &opts, languages, ctx),
        CheckKind::Toolchain => toolchain::run(rule, &opts, ctx),
        CheckKind::PolicyTightening => tightening::run(rule, &opts, ctx),
        CheckKind::Command => command::run(rule, &opts, ctx),
    }
}
