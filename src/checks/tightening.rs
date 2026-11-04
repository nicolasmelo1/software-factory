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
    Finding::new(
        &rule.id,
        rule.severity,
        POLICY_PATH,
        key.to_string(),
        message,
    )
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
            Some(current) => findings.extend(rule_findings(
                rule,
                ctx.catalog,
                crate::policy::merge,
                id,
                previous,
                current,
            )),
        }
    }
    findings.extend(project_findings(rule, before, ctx.policy));
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

/// What the policy's `project` loses: a default branch dropped means a run
/// without `--changed` stops reading the work's diff, and a plan's changed
/// files go back to being whatever its author remembered.
fn project_findings(rule: &Rule, before: &Policy, now: &Policy) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(branch) = &before.project.default_branch
        && now.project.default_branch.is_none()
    {
        findings.push(weakened(
            rule,
            "default-branch-removed",
            format!(
                "`project.default_branch` was removed, so a run without `--changed` no longer reads the work's diff against `{branch}`"
            ),
            format!("default_branch: {branch}"),
            "none".to_string(),
        ));
    }
    findings
}

/// How an instance's options fall through to the catalog default. Passed in
/// rather than called by name so the property in this file's tests can
/// compare through a merge with a defect planted, which is how it shows it
/// can fail.
type Merge = fn(&serde_yaml::Value, &serde_yaml::Value) -> serde_yaml::Value;

/// One rule, compared with the version being replaced.
fn rule_findings(
    rule: &Rule,
    catalog: &crate::catalog::Catalog,
    merge: Merge,
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
    let (was, now) = (
        effective_size(catalog, merge, id, &previous.options),
        effective_size(catalog, merge, id, &current.options),
    );
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
        (
            "forbidden_in_goal",
            was.forbidden_in_goal,
            now.forbidden_in_goal,
        ),
        (
            "forbidden_actors",
            was.forbidden_actors,
            now.forbidden_actors,
        ),
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
    findings.extend(declaration_findings(rule, id, &was, &now));
    findings
}

/// What a plan declaration's options lose: a ceiling raised or removed, a
/// declaration no longer required, crossings no longer derived.
fn declaration_findings(rule: &Rule, id: &str, was: &Size, now: &Size) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (name, was_max, now_max) in [
        ("max_words", was.max_words, now.max_words),
        ("max_body_lines", was.max_body_lines, now.max_body_lines),
    ] {
        let Some(was_max) = was_max else { continue };
        if now_max.is_none_or(|now_max| now_max > was_max) {
            findings.push(weakened(
                rule,
                &format!("raised:{id}:{name}"),
                format!("{id} had its {name} ceiling raised or removed"),
                was_max.to_string(),
                now_max.map_or("none".to_string(), |n| n.to_string()),
            ));
        }
    }
    if was.require_declaration && !now.require_declaration {
        findings.push(weakened(
            rule,
            &format!("undeclared-allowed:{id}"),
            format!("{id} no longer refuses a plan with no declaration"),
            "require_declaration: true".to_string(),
            "require_declaration: false".to_string(),
        ));
    }
    if now.incidental > was.incidental {
        findings.push(weakened(
            rule,
            &format!("incidental-widened:{id}"),
            format!(
                "{id} lets {} more glob(s) of changed files go undeclared",
                now.incidental - was.incidental
            ),
            format!("{} incidental glob(s)", was.incidental),
            format!("{} incidental glob(s)", now.incidental),
        ));
    }
    if now.delivered < was.delivered {
        findings.push(weakened(
            rule,
            &format!("delivered-narrowed:{id}"),
            format!("{id} reads delivered plans in fewer places"),
            format!("{} delivered glob(s)", was.delivered),
            format!("{} delivered glob(s)", now.delivered),
        ));
    }
    if was.mechanisms.is_some() && now.mechanisms.is_none() {
        findings.push(weakened(
            rule,
            &format!("uncatalogued:{id}"),
            format!("{id} no longer derives crossings from a mechanisms catalog"),
            was.mechanisms.clone().unwrap_or_default(),
            "none".to_string(),
        ));
    }
    findings
}

fn ratchet_findings(
    rule: &Rule,
    ctx: &Ctx,
    before_policy: &Policy,
    before: &Ratchet,
) -> Vec<Finding> {
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
                        added
                            .iter()
                            .take(3)
                            .map(|k| k.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
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
    max_words: Option<usize>,
    max_body_lines: Option<usize>,
    require_declaration: bool,
    mechanisms: Option<String>,
    incidental: usize,
    delivered: usize,
}

/// The counts that decide direction, computed on the options a rule will
/// actually run under: the instance's `options:` shallow-merged over the
/// catalog default, exactly as `checks::options_for` merges them.
///
/// The raw key count reported a false weakening whenever a policy re-declared
/// a default to keep it: widening `L4.PLAN_PROOF_BUDGET`'s scope has to carry
/// its default exclude along (instance options replace rather than extend),
/// and the raw count read that as an exclusion gained when the effective set
/// grew by none.
fn effective_size(
    catalog: &crate::catalog::Catalog,
    merge: Merge,
    id: &str,
    options: &serde_yaml::Value,
) -> Size {
    let defaults = catalog
        .get(crate::policy::base_rule_id(id))
        .map(|r| r.defaults.clone())
        .unwrap_or(serde_yaml::Value::Null);
    let merged = merge(&defaults, options);
    let options: Options = serde_yaml::from_value(merged).unwrap_or_default();
    Size {
        excludes: options.exclude.len(),
        scope: options.scope.len(),
        max: options.max,
        forbidden_in_goal: options.forbidden_in_goal.len(),
        forbidden_actors: options.forbidden_actors.len(),
        max_words: options.max_words,
        max_body_lines: options.max_body_lines,
        require_declaration: options.require_declaration,
        mechanisms: options.mechanisms,
        incidental: options.incidental.len(),
        delivered: options.delivered.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::{effective_size, previously_enabled, rule_findings};
    use crate::catalog::Catalog;
    use crate::policy::{Policy, RuleSetting};

    #[test]
    fn redeclaring_a_catalog_default_is_not_a_new_exclusion() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let previous: RuleSetting = serde_yaml::from_str("enabled: true\n").expect("parses");
        let current: RuleSetting = serde_yaml::from_str(
            "enabled: true\noptions:\n  scope: [docs/design/*.md]\n  exclude: [plans/next-steps.md]\n",
        )
        .expect("parses");
        assert!(
            rule_findings(
                catalog.get("L2.POLICY_ONLY_TIGHTENS").expect("ships"),
                &catalog,
                crate::policy::merge,
                "L4.PLAN_PROOF_BUDGET",
                &previous,
                &current,
            )
            .is_empty(),
            "the exclude was already the catalog default; widening scope with it carried is not a weakening"
        );
    }

    #[test]
    fn dropping_a_catalog_default_exclusion_is_not_a_weakening() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let previous: RuleSetting =
            serde_yaml::from_str("enabled: true\noptions:\n  exclude: [plans/next-steps.md]\n")
                .expect("parses");
        let current: RuleSetting = serde_yaml::from_str("enabled: true\n").expect("parses");
        assert!(
            rule_findings(
                catalog.get("L2.POLICY_ONLY_TIGHTENS").expect("ships"),
                &catalog,
                crate::policy::merge,
                "L4.PLAN_PROOF_BUDGET",
                &previous,
                &current,
            )
            .is_empty(),
            "the dropped exclude is the catalog default, so the effective set is unchanged — the raw key count could not see that"
        );
    }

    #[test]
    fn dropping_an_exclusion_is_a_tightening_and_stays_quiet() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let previous: RuleSetting =
            serde_yaml::from_str("enabled: true\noptions:\n  exclude: [src/legacy/**]\n")
                .expect("parses");
        let current: RuleSetting = serde_yaml::from_str("enabled: true\n").expect("parses");
        assert!(
            rule_findings(
                catalog.get("L2.POLICY_ONLY_TIGHTENS").expect("ships"),
                &catalog,
                crate::policy::merge,
                "L4.PLAN_PROOF_BUDGET",
                &previous,
                &current,
            )
            .is_empty(),
            "fewer exclusions means more findings, which is the direction this rule never taxes"
        );
    }

    #[test]
    fn effective_size_merges_over_the_catalog_default() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let size = effective_size(
            &catalog,
            crate::policy::merge,
            "L4.PLAN_PROOF_BUDGET",
            &serde_yaml::Value::Null,
        );
        assert_eq!(
            size.excludes, 1,
            "the default exclude survives a null override"
        );
        assert_eq!(
            size.max,
            Some(60),
            "the default budget survives a null override"
        );
    }

    #[test]
    fn loosening_a_plan_declaration_is_a_weakening() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rule = catalog
            .get("L2.POLICY_ONLY_TIGHTENS")
            .expect("the rule ships");
        let previous: RuleSetting = serde_yaml::from_str(
            "enabled: true\noptions:\n  mechanisms: docs/mechanisms.yaml\n  max_words: 40\n  max_body_lines: 6\n  require_declaration: true\n",
        )
        .expect("the previous setting parses");
        let current: RuleSetting =
            serde_yaml::from_str("enabled: true\noptions:\n  max_words: 80\n")
                .expect("the current setting parses");
        let id = "L4.PLAN_IS_A_LINKED_DECLARATION";
        let mut keys: Vec<_> = rule_findings(
            rule,
            &catalog,
            crate::policy::merge,
            id,
            &previous,
            &current,
        )
        .into_iter()
        .map(|finding| finding.key)
        .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                format!("raised:{id}:max_body_lines"),
                format!("raised:{id}:max_words"),
                format!("uncatalogued:{id}"),
                format!("undeclared-allowed:{id}"),
            ]
        );
        assert!(
            rule_findings(
                rule,
                &catalog,
                crate::policy::merge,
                id,
                &current,
                &previous
            )
            .is_empty(),
            "tightening every declaration option is silent"
        );
    }

    #[test]
    fn widening_incidental_is_a_weakening_and_narrowing_it_is_not() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rule = catalog
            .get("L2.POLICY_ONLY_TIGHTENS")
            .expect("the rule ships");
        let id = "L4.PLAN_IS_A_LINKED_DECLARATION";
        let previous: RuleSetting = serde_yaml::from_str("enabled: true\n").expect("parses");
        let current: RuleSetting = serde_yaml::from_str(
            "enabled: true\noptions:\n  incidental: [\".software-factory/**\", \"plans/next-steps.md\", \"src/**\"]\n  delivered: []\n",
        )
        .expect("parses");
        let keys = |was: &RuleSetting, now: &RuleSetting| {
            let mut keys: Vec<_> =
                rule_findings(rule, &catalog, crate::policy::merge, id, was, now)
                    .into_iter()
                    .map(|finding| finding.key)
                    .collect();
            keys.sort();
            keys
        };
        assert_eq!(
            keys(&previous, &current),
            vec![
                format!("delivered-narrowed:{id}"),
                format!("incidental-widened:{id}"),
            ]
        );
        assert!(
            keys(&current, &previous).is_empty(),
            "back to the catalog default is a tightening"
        );
    }

    #[test]
    fn dropping_the_default_branch_is_a_weakening() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rule = catalog
            .get("L2.POLICY_ONLY_TIGHTENS")
            .expect("the rule ships");
        let with: Policy = serde_yaml::from_str(
            "version: 1\nproject:\n  name: p\n  languages: []\n  default_branch: main\n",
        )
        .expect("parses");
        let without: Policy =
            serde_yaml::from_str("version: 1\nproject:\n  name: p\n  languages: []\n")
                .expect("parses");
        let keys: Vec<_> = super::project_findings(rule, &with, &without)
            .into_iter()
            .map(|finding| finding.key)
            .collect();
        assert_eq!(keys, vec!["default-branch-removed"]);
        assert!(super::project_findings(rule, &without, &with).is_empty());
    }

    #[test]
    fn removing_goal_or_actor_denylist_values_is_a_weakening() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        let rule = catalog
            .get("L2.POLICY_ONLY_TIGHTENS")
            .expect("the rule ships");
        let previous: RuleSetting = serde_yaml::from_str(
            "enabled: true\noptions:\n  forbidden_in_goal: [/Users/]\n  forbidden_actors: [scripted]\n",
        )
        .expect("the previous setting parses");
        let current: RuleSetting = serde_yaml::from_str(
            "enabled: true\noptions:\n  forbidden_in_goal: []\n  forbidden_actors: []\n",
        )
        .expect("the current setting parses");

        let keys: Vec<_> = rule_findings(
            rule,
            &catalog,
            crate::policy::merge,
            "L3.GATE_HAS_FRESH_EVIDENCE",
            &previous,
            &current,
        )
        .into_iter()
        .map(|finding| finding.key)
        .collect();
        assert!(keys.contains(
            &"denylist-reduced:L3.GATE_HAS_FRESH_EVIDENCE:forbidden_in_goal".to_string()
        ));
        assert!(
            keys.contains(
                &"denylist-reduced:L3.GATE_HAS_FRESH_EVIDENCE:forbidden_actors".to_string()
            )
        );
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

/// `L2.POLICY_ONLY_TIGHTENS` over every starting value rather than three: a
/// weakening move is reported and its inverse is silent, whatever explicit
/// value the policy started from — including the `0`, `false` and empty list
/// a merge is most likely to read as unset.
///
/// Its falsifier is `falsy_read_as_unset`: a merge that treats an explicit
/// zero, `false` or empty override as absent, so the catalog default shows
/// through. Planted when `SF_FALSIFIER` names it, which is how
/// `L5.EVERY_PROPERTY_HAS_A_FALSIFIER` proves this property can fail.
#[cfg(test)]
mod property {
    use super::{Merge, rule_findings};
    use crate::catalog::Catalog;
    use crate::policy::RuleSetting;
    use proptest::prelude::*;
    use proptest::test_runner::Config;
    use serde_yaml::{Mapping, Value};

    /// Loaded once: the catalog compiles every query it ships, and a case
    /// that reloads it spends its time there rather than on the property.
    static CATALOG: std::sync::LazyLock<Catalog> =
        std::sync::LazyLock::new(|| Catalog::builtin().expect("the shipped catalog loads"));

    /// The planted defect, the one config merges keep shipping: an override
    /// that is false, zero or empty reads as "not set".
    fn falsy_read_as_unset(base: &Value, over: &Value) -> Value {
        let Value::Mapping(over) = over else {
            return crate::policy::merge(base, over);
        };
        let set: Mapping = over
            .iter()
            .filter(|(_, value)| match value {
                Value::Bool(b) => *b,
                Value::Number(n) => n.as_u64() != Some(0),
                Value::Sequence(items) => !items.is_empty(),
                Value::String(s) => !s.is_empty(),
                _ => true,
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        crate::policy::merge(base, &Value::Mapping(set))
    }

    /// The merge under test: the real one, unless a falsifier is planted.
    fn merge() -> Merge {
        match std::env::var(crate::checks::properties::FALSIFIER_VAR).as_deref() {
            Ok("falsy_read_as_unset") => falsy_read_as_unset,
            _ => crate::policy::merge,
        }
    }

    /// One rule instance's explicit options: every value the comparison
    /// reads, each stated rather than left to the default.
    #[derive(Debug, Clone)]
    struct Setting {
        enabled: bool,
        max: u64,
        exclude: Vec<String>,
        scope: Vec<String>,
        max_words: u64,
        max_body_lines: u64,
        require_declaration: bool,
        forbidden_in_goal: Vec<String>,
        incidental: Vec<String>,
    }

    impl Setting {
        fn rule_setting(&self) -> RuleSetting {
            let options = serde_yaml::to_value(serde_json::json!({
                "max": self.max,
                "exclude": self.exclude,
                "scope": self.scope,
                "max_words": self.max_words,
                "max_body_lines": self.max_body_lines,
                "require_declaration": self.require_declaration,
                "forbidden_in_goal": self.forbidden_in_goal,
                "incidental": self.incidental,
            }))
            .expect("options serialise");
            RuleSetting {
                enabled: self.enabled,
                options,
                when: None,
            }
        }
    }

    /// One move in the direction the rule refuses.
    #[derive(Debug, Clone, Copy)]
    enum Weakening {
        Disable,
        Exclude,
        RaiseMax(u64),
        NarrowScope,
        RaiseMaxWords(u64),
        RaiseMaxBodyLines(u64),
        StopRequiringDeclaration,
        AdmitGoal,
        WidenIncidental,
    }

    impl Weakening {
        /// The setting after the move, or `None` where the move has nothing
        /// to take away.
        fn apply(self, before: &Setting) -> Option<Setting> {
            let mut after = before.clone();
            match self {
                Weakening::Disable => after.enabled = false,
                Weakening::Exclude => after.exclude.push("weakened/**".to_string()),
                Weakening::RaiseMax(by) => after.max += by,
                Weakening::NarrowScope => {
                    after.scope.pop()?;
                }
                Weakening::RaiseMaxWords(by) => after.max_words += by,
                Weakening::RaiseMaxBodyLines(by) => after.max_body_lines += by,
                Weakening::StopRequiringDeclaration if before.require_declaration => {
                    after.require_declaration = false
                }
                Weakening::StopRequiringDeclaration => return None,
                Weakening::AdmitGoal => {
                    after.forbidden_in_goal.pop()?;
                }
                Weakening::WidenIncidental => after.incidental.push("widened/**".to_string()),
            }
            Some(after)
        }
    }

    /// Zero and empty half the time: the boundary is where merges lose a
    /// value, so the generator is pointed at it rather than hoping.
    fn count() -> impl Strategy<Value = u64> {
        prop_oneof![Just(0u64), 1u64..80]
    }

    fn globs(prefix: &'static str) -> impl Strategy<Value = Vec<String>> {
        prop_oneof![
            Just(Vec::new()),
            prop::collection::vec((0u8..9).prop_map(move |n| format!("{prefix}{n}/**")), 1..4),
        ]
    }

    fn setting() -> impl Strategy<Value = Setting> {
        (
            count(),
            globs("excluded"),
            globs("scoped"),
            count(),
            count(),
            any::<bool>(),
            globs("/goal"),
            globs("incidental"),
        )
            .prop_map(
                |(max, exclude, scope, max_words, max_body_lines, require, goal, incidental)| {
                    Setting {
                        enabled: true,
                        max,
                        exclude,
                        scope,
                        max_words,
                        max_body_lines,
                        require_declaration: require,
                        forbidden_in_goal: goal,
                        incidental,
                    }
                },
            )
    }

    fn weakening() -> impl Strategy<Value = Weakening> {
        prop_oneof![
            Just(Weakening::Disable),
            Just(Weakening::Exclude),
            (1u64..5).prop_map(Weakening::RaiseMax),
            Just(Weakening::NarrowScope),
            (1u64..5).prop_map(Weakening::RaiseMaxWords),
            (1u64..5).prop_map(Weakening::RaiseMaxBodyLines),
            Just(Weakening::StopRequiringDeclaration),
            Just(Weakening::AdmitGoal),
            Just(Weakening::WidenIncidental),
        ]
    }

    /// Rules whose catalog defaults differ: a ceiling and a scope, a ceiling
    /// alone, and none, so a default showing through is visible.
    fn rule_id() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("L4.PLAN_PROOF_BUDGET"),
            Just("L1.COMPLEXITY_CEILING"),
            Just("L4.PLAN_IS_A_LINKED_DECLARATION"),
            Just("L3.GATE_HAS_FRESH_EVIDENCE"),
        ]
    }

    proptest! {
        // No regression file: a falsifier run is meant to fail, and must not
        // write into the tree it is checking.
        #![proptest_config(Config { cases: 512, failure_persistence: None, ..Config::default() })]

        #[test]
        fn a_weakening_is_reported_and_its_inverse_is_silent(
            id in rule_id(),
            before in setting(),
            weakening in weakening(),
        ) {
            let Some(after) = weakening.apply(&before) else {
                return Err(TestCaseError::reject("nothing to take away"));
            };
            let catalog = &*CATALOG;
            let rule = catalog.get("L2.POLICY_ONLY_TIGHTENS").expect("the rule ships");
            let (was, now) = (before.rule_setting(), after.rule_setting());
            prop_assert!(
                !rule_findings(rule, catalog, merge(), id, &was, &now).is_empty(),
                "{weakening:?} on {id} from {before:?} passed silently"
            );
            prop_assert!(
                rule_findings(rule, catalog, merge(), id, &now, &was).is_empty(),
                "undoing {weakening:?} on {id} was reported as a weakening"
            );
        }
    }
}
