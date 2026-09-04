//! L2 — the guardrail may be strengthened, never quietly weakened.
//!
//! The lock rules make an edit to the factory's own configuration visible.
//! This one reads the edit and decides which direction it went. Disabling a
//! rule, widening an exclusion, adding a ratchet key or pushing a review date
//! out are all indistinguishable from a fix in a diff summary, and they are
//! the specific moves an agent makes when a check is between it and a green
//! build.
//!
//! Tightening — enabling a rule, narrowing a scope, removing a frozen entry —
//! passes silently. Only the weakening direction needs a human.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::policy::{Options, POLICY_PATH, Policy, RATCHET_PATH};
use crate::ratchet::Ratchet;
use anyhow::Result;
use std::process::Command;

/// The previous policy and ratchet, from git or from a checked-in baseline.
fn baseline(ctx: &Ctx, opts: &Options) -> Result<Option<(Policy, Ratchet)>> {
    match &opts.baseline {
        Some(dir) => from_directory(ctx, dir),
        // No baseline to compare against. The lock rules still cover the fact
        // that the file changed; this rule simply has nothing to say.
        None => match &ctx.base {
            Some(base) => from_git(ctx, base),
            None => Ok(None),
        },
    }
}

fn from_directory(ctx: &Ctx, dir: &str) -> Result<Option<(Policy, Ratchet)>> {
    let root = ctx.root.join(dir);
    if !root.join(POLICY_PATH).exists() {
        return Ok(None);
    }
    Ok(Some((Policy::load(&root)?, Ratchet::load(&root)?)))
}

fn from_git(ctx: &Ctx, base: &str) -> Result<Option<(Policy, Ratchet)>> {
    let policy = match show(ctx, base, POLICY_PATH)? {
        Some(body) => serde_yaml::from_str(&body)?,
        None => return Ok(None),
    };
    let ratchet = match show(ctx, base, RATCHET_PATH)? {
        Some(body) => serde_yaml::from_str(&body)?,
        None => Ratchet::default(),
    };
    Ok(Some((policy, ratchet)))
}

fn show(ctx: &Ctx, base: &str, path: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(ctx.root)
        .args(["show", &format!("{base}:{path}")])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let Some((before, frozen_before)) = baseline(ctx, opts)? else {
        return Ok(Vec::new());
    };
    let mut findings = policy_findings(rule, ctx, &before);
    findings.extend(ratchet_findings(rule, ctx, &before, &frozen_before));
    Ok(findings)
}

fn weakened(rule: &Rule, key: &str, message: String, expected: String, actual: String) -> Finding {
    Finding::new(&rule.id, rule.severity, POLICY_PATH, key.to_string(), message)
        .expected(expected)
        .actual(actual)
}

fn policy_findings(rule: &Rule, ctx: &Ctx, before: &Policy) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, previous) in &before.rules {
        match ctx.policy.rules.get(id) {
            None => findings.push(weakened(
                rule,
                &format!("removed:{id}"),
                format!("{id} was removed from the policy"),
                "the rule still present".to_string(),
                "removed".to_string(),
            )),
            Some(current) => findings.extend(rule_findings(rule, id, previous, current)),
        }
    }
    for (name, previous) in &before.gates {
        match ctx.policy.gates.get(name) {
            None => findings.push(weakened(
                rule,
                &format!("gate-removed:{name}"),
                format!("gate `{name}` was removed"),
                "the gate still present".to_string(),
                "removed".to_string(),
            )),
            Some(current) if current.activation.len() < previous.activation.len() => {
                findings.push(weakened(
                    rule,
                    &format!("gate-narrowed:{name}"),
                    format!("gate `{name}` now activates from fewer paths"),
                    format!("{} activation path(s)", previous.activation.len()),
                    format!("{} activation path(s)", current.activation.len()),
                ))
            }
            Some(_) => {}
        }
    }
    findings
}

/// One rule, compared with the version being replaced.
fn rule_findings(
    rule: &Rule,
    id: &str,
    previous: &crate::policy::RuleSetting,
    current: &crate::policy::RuleSetting,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if previous.enabled && !current.enabled {
        findings.push(weakened(
            rule,
            &format!("disabled:{id}"),
            format!("{id} was switched off"),
            "enabled: true".to_string(),
            "enabled: false".to_string(),
        ));
    }
    let (was, now) = (option_size(&previous.options), option_size(&current.options));
    if now.excludes > was.excludes {
        findings.push(weakened(
            rule,
            &format!("excluded:{id}"),
            format!("{id} gained {} exclusion(s)", now.excludes - was.excludes),
            format!("{} exclusion(s)", was.excludes),
            format!("{} exclusion(s)", now.excludes),
        ));
    }
    if was.scope > 0 && now.scope < was.scope {
        findings.push(weakened(
            rule,
            &format!("narrowed:{id}"),
            format!("{id} now covers less than it did"),
            format!("{} scope entr(ies)", was.scope),
            format!("{} scope entr(ies)", now.scope),
        ));
    }
    if let (Some(was_max), Some(now_max)) = (was.max, now.max)
        && now_max > was_max
    {
        findings.push(weakened(
            rule,
            &format!("raised:{id}"),
            format!("{id} had its ceiling raised"),
            was_max.to_string(),
            now_max.to_string(),
        ));
    }
    for (name, was_count, now_count) in [
        ("forbidden_in_goal", was.forbidden_in_goal, now.forbidden_in_goal),
        ("forbidden_actors", was.forbidden_actors, now.forbidden_actors),
    ] {
        if now_count < was_count {
            findings.push(weakened(
                rule,
                &format!("denylist-reduced:{id}:{name}"),
                format!("{id} removed {} {name} value(s)", was_count - now_count),
                format!("{was_count} value(s)"),
                format!("{now_count} value(s)"),
            ));
        }
    }
    findings
}

fn ratchet_findings(rule: &Rule, ctx: &Ctx, before_policy: &Policy, before: &Ratchet) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, current) in &ctx.ratchet.rules {
        // A new rule is a strengthening, but it may expose debt already in
        // the repository. Its first ratchet is the adoption baseline, not a
        // newly frozen violation. Once the rule has been enabled, every new
        // key remains the quiet weakening this check must refuse.
        if !previously_enabled(before_policy, id) {
            continue;
        }
        let previous = before.rules.get(id);
        let previously_frozen = previous.map(|p| p.allow.len()).unwrap_or(0);
        let added: Vec<&String> = current
            .allow
            .iter()
            .filter(|key| !previous.is_some_and(|p| p.allow.contains(*key)))
            .collect();
        if !added.is_empty() {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    RATCHET_PATH,
                    format!("frozen:{id}"),
                    format!(
                        "{id} froze {} new violation(s): {}",
                        added.len(),
                        added.iter().take(3).map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                )
                .expected(format!("at most the {previously_frozen} already frozen"))
                .actual(format!("{} frozen", current.allow.len())),
            );
        }
        if let Some(previous) = previous
            && current.review_by > previous.review_by
        {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    RATCHET_PATH,
                    format!("deferred:{id}"),
                    format!("{id} had its review date pushed out"),
                )
                .expected(previous.review_by.clone())
                .actual(current.review_by.clone()),
            );
        }
    }
    findings
}

fn previously_enabled(policy: &Policy, id: &str) -> bool {
    policy.rules.get(id).is_some_and(|setting| setting.enabled)
}

struct Size {
    excludes: usize,
    scope: usize,
    max: Option<usize>,
    forbidden_in_goal: usize,
    forbidden_actors: usize,
}

fn option_size(raw: &serde_yaml::Value) -> Size {
    let options: Options = serde_yaml::from_value(raw.clone()).unwrap_or_default();
    Size {
        excludes: options.exclude.len(),
        scope: options.scope.len(),
        max: options.max,
        forbidden_in_goal: options.forbidden_in_goal.len(),
        forbidden_actors: options.forbidden_actors.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::{previously_enabled, rule_findings};
    use crate::catalog::Catalog;
    use crate::policy::{Policy, RuleSetting};

    #[test]
    fn removing_goal_or_actor_denylist_values_is_a_weakening() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rule = catalog.get("L2.POLICY_ONLY_TIGHTENS").expect("the rule ships");
        let previous: RuleSetting = serde_yaml::from_str(
            "enabled: true\noptions:\n  forbidden_in_goal: [/Users/]\n  forbidden_actors: [scripted]\n",
        )
        .expect("the previous setting parses");
        let current: RuleSetting = serde_yaml::from_str(
            "enabled: true\noptions:\n  forbidden_in_goal: []\n  forbidden_actors: []\n",
        )
        .expect("the current setting parses");

        let keys: Vec<_> = rule_findings(rule, "L3.GATE_HAS_FRESH_EVIDENCE", &previous, &current)
            .into_iter()
            .map(|finding| finding.key)
            .collect();
        assert!(keys.contains(&"denylist-reduced:L3.GATE_HAS_FRESH_EVIDENCE:forbidden_in_goal".to_string()));
        assert!(keys.contains(&"denylist-reduced:L3.GATE_HAS_FRESH_EVIDENCE:forbidden_actors".to_string()));
    }

    #[test]
    fn an_initial_ratchet_seed_is_allowed_only_for_a_newly_enabled_rule() {
        let before: Policy = serde_yaml::from_str(
            "version: 1\nproject:\n  name: before\n  languages: [rust]\nrules:\n  L1.COMMENT_STAYS_SUCCINCT:\n    enabled: true\n  L4.PLAN_PROOF_BUDGET:\n    enabled: false\n",
        )
        .expect("the baseline policy parses");

        assert!(previously_enabled(&before, "L1.COMMENT_STAYS_SUCCINCT"));
        assert!(
            !previously_enabled(&before, "L4.PLAN_PROOF_BUDGET"),
            "a previously disabled rule may seed the debt it exposes when enabled"
        );
        assert!(
            !previously_enabled(&before, "L1.INDIRECTION_EARNS_ITS_NAME"),
            "a rule absent from the baseline is newly enabled too"
        );
    }
}
