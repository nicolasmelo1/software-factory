//! L2 — hash locks and dated exceptions.
//!
//! A lock is a permission gate, not a cache. It exists so that changing a
//! derived artifact costs a visible, reviewable line in a second file.

use super::Ctx;
use crate::catalog::Rule;
use crate::digest;
use crate::finding::{Finding, Severity};
use crate::policy::Options;
use crate::scan;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Lock {
    pub schema_version: u32,
    /// Repo-relative path -> SHA-256.
    pub files: BTreeMap<String, String>,
}

impl Lock {
    pub fn load(path: &Path) -> Result<Option<Lock>> {
        if !path.exists() {
            return Ok(None);
        }
        let body = std::fs::read_to_string(path)?;
        let lock: Lock = serde_json::from_str(&body)
            .with_context(|| format!("{} is not a valid lock", path.display()))?;
        anyhow::ensure!(lock.schema_version == 1, "unsupported lock schema");
        Ok(Some(lock))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(self)?))?;
        Ok(())
    }
}

/// Recompute the lock a rule's scope should currently have.
pub fn current(opts: &Options, ctx: &Ctx) -> Result<Lock> {
    let mut files = BTreeMap::new();
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        files.insert(file.rel.clone(), digest::file(&file.abs)?);
    }
    Ok(Lock { schema_version: 1, files })
}

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    if opts.scope.is_empty() {
        return Ok(Vec::new()); // nothing declared locked in this repo
    }
    let lock_path = ctx.root.join(
        opts.lock_file
            .as_deref()
            .unwrap_or(".software-factory/locks/default.lock.json"),
    );
    let observed = current(opts, ctx)?;
    let Some(locked) = Lock::load(&lock_path)? else {
        return Ok(vec![Finding::new(
            &rule.id,
            rule.severity,
            lock_path.display().to_string(),
            "missing-lock".to_string(),
            "this rule is enabled but its lock has never been written",
        )
        .expected("a lock file")
        .actual("missing — run `sf lock --update`")]);
    };

    let mut findings = Vec::new();
    for (path, expected) in &locked.files {
        match observed.files.get(path) {
            None => findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    path.clone(),
                    path.clone(),
                    "a locked file was deleted or moved",
                )
                .expected(expected.clone())
                .actual("missing".to_string()),
            ),
            Some(actual) if actual != expected => findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    path.clone(),
                    path.clone(),
                    "a locked file was modified without updating the lock",
                )
                .expected(expected.clone())
                .actual(actual.clone()),
            ),
            Some(_) => {}
        }
    }
    for path in observed.files.keys() {
        if !locked.files.contains_key(path) {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    path.clone(),
                    path.clone(),
                    "a file appeared in locked scope without being declared",
                )
                .expected("an entry in the lock")
                .actual("undeclared".to_string()),
            );
        }
    }
    Ok(findings)
}

/// `L2.NO_PERMANENT_EXCEPTION`: every frozen set carries a future review date.
pub fn expiry(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (rule_id, entry) in &ctx.ratchet.rules {
        if entry.allow.is_empty() {
            continue;
        }
        let location = format!("{}#{rule_id}", crate::policy::RATCHET_PATH);
        if entry.review_by.trim().is_empty() {
            findings.push(
                Finding::new(
                    &rule.id,
                    Severity::High,
                    location,
                    rule_id.clone(),
                    format!("{rule_id} freezes {} violations with no review date", entry.allow.len()),
                )
                .expected("review_by: YYYY-MM-DD"),
            );
        } else if entry.review_by.as_str() < ctx.today.as_str() {
            findings.push(
                Finding::new(
                    &rule.id,
                    Severity::High,
                    location,
                    rule_id.clone(),
                    format!(
                        "{rule_id} still freezes {} violations past its review date",
                        entry.allow.len()
                    ),
                )
                .expected(format!("a date on or after {}", ctx.today))
                .actual(entry.review_by.clone()),
            );
        }
    }
    Ok(findings)
}
