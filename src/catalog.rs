//! The rule catalog: the actual portable asset.
//!
//! A rule is prose (statement / why / fix) plus a machine-checkable spec.
//! Both halves are mandatory. A rule with no `why` is a rule nobody can
//! argue with, and a rule with no spec is a rule nothing enforces — the
//! catalog refuses to load either.

use crate::finding::Severity;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum Layer {
    /// Shape — where things live.
    L0,
    /// Grain — how code reads, and what escape hatches are banned.
    L1,
    /// Contract — no drift between a source of truth and what derives from it.
    L2,
    /// Effect — a real actor achieved the observable outcome.
    L3,
    /// Cadence — how docs, plans and phases connect.
    L4,
    /// Meta — the guardrail is itself proven to fire.
    L5,
    /// Hazard — the classes of defect this repository actively hunts, and
    /// proof that the hunt runs.
    L6,
}

impl Layer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Layer::L0 => "L0",
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::L5 => "L5",
            Layer::L6 => "L6",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LangQuery {
    /// A tree-sitter query. It must bind `@target`, and may bind `@name`.
    pub query: String,
    /// The same shape, matched only where it is accompanied by what makes it
    /// acceptable — a `@target` here cancels the one `query` found on that
    /// line. A tree-sitter query cannot say "unless a comment sits above
    /// this", because negation over siblings is not expressible; two positive
    /// queries and a set difference say it, and stay readable.
    #[serde(default)]
    pub unless: Option<String>,
}

/// A containment rule: `inner` must not appear anywhere inside `outer`.
/// This is what makes the concurrency hazards checkable — "no blocking call
/// while holding a lock" is a statement about nesting, not about placement.
#[derive(Debug, Clone, Deserialize)]
pub struct NestedQuery {
    pub outer: String,
    pub inner: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RatchetPolicy {
    /// Existing violations may be frozen by key in `.software-factory/ratchet.yaml`.
    #[default]
    Allowlist,
    /// No grandfathering: this rule is not adoptable halfway.
    None,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckKind {
    /// Structural placement of AST nodes. L0.
    Shape {
        #[serde(default)]
        languages: BTreeMap<String, LangQuery>,
    },
    /// Cyclomatic ceiling per function. L1.
    Complexity,
    /// Blanket escape hatches and unreasoned suppressions. L1.
    TextPattern,
    /// Hash manifest over generated / vendored / dependency-declaring files. L2.
    Lock,
    /// Every frozen exception carries a future `review_by`. L2.
    Expiry,
    /// Documentation, plans and rules stay attached to each other. L4/L5.
    Cadence { mode: CadenceMode },
    /// A gate activated by touched paths has digest-verified, non-stale evidence. L3.
    Evidence,
    /// A node kind that must not appear inside another. L6.
    Nested {
        #[serde(default)]
        languages: BTreeMap<String, NestedQuery>,
    },
    /// A hazard the repository declares it hunts must have a tool that runs. L6.
    Toolchain,
    /// The policy and the ratchet may be strengthened, never weakened. L2.
    PolicyTightening,
    /// A check this tool cannot express, run as a command. L2.
    Command,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CadenceMode {
    /// Repo-relative markdown links resolve.
    DocLinks,
    /// No undeclared top-level files.
    RootFiles,
    /// Every enabled rule is cited in prose, and every citation resolves.
    RuleCitations,
    /// Every plan declares an exit condition and sits in the execution order.
    PlanCadence,
    /// Every acceptance criterion in a plan names the check that proves it.
    PlanCriteria,
    /// Every check a plan's criteria name is one its gate actually requires.
    GateCoverage,
    /// Every enabled rule has a mutation fixture that proves the check fires.
    MutationCoverage,
    /// No enabled rule is configured so it cannot produce a finding.
    InertRules,
    /// Every `sf` invocation an enabled rule's prose quotes is one this
    /// binary accepts.
    RuleCommands,
    /// Every promise a page marks is joined to a gate the policy declares.
    ClaimCitations,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub id: String,
    pub layer: Layer,
    pub title: String,
    /// What the rule requires, in one sentence, in the imperative.
    pub statement: String,
    /// Why it exists. This is what the agent reads when the check fails.
    pub why: String,
    /// What to do about a violation.
    pub fix: String,
    pub severity: Severity,
    #[serde(default)]
    pub ratchet: RatchetPolicy,
    pub check: CheckKind,
    /// Option defaults, overridable per repo in `policy.yaml`.
    #[serde(default)]
    pub defaults: serde_yaml::Value,
}

impl Rule {
    /// Compile everything the rule will need at check time. A rule with a
    /// broken query or regex must fail on load, not halfway through a run
    /// that has already written files.
    pub fn validate(&self) -> Result<()> {
        let options: crate::policy::Options = serde_yaml::from_value(self.defaults.clone())
            .context("`defaults` do not match the option schema")?;
        validate_patterns(&options)?;
        match &self.check {
            CheckKind::Shape { languages } => validate_queries(languages)?,
            CheckKind::Nested { languages } => {
                for (name, spec) in languages {
                    validate_query(name, &spec.outer)?;
                    validate_query(name, &spec.inner)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn validate_patterns(options: &crate::policy::Options) -> Result<()> {
    for pattern in &options.forbidden {
        regex::Regex::new(&pattern.regex)
            .with_context(|| format!("invalid regex {:?}", pattern.regex))?;
        if let Some(unless) = &pattern.unless {
            regex::Regex::new(unless).with_context(|| format!("invalid `unless` regex {unless:?}"))?;
        }
    }
    if let Some(marker) = &options.marker {
        regex::Regex::new(marker).with_context(|| format!("invalid marker {marker:?}"))?;
    }
    Ok(())
}

fn validate_queries(languages: &BTreeMap<String, LangQuery>) -> Result<()> {
    for (name, spec) in languages {
        validate_query(name, &spec.query)?;
        if let Some(unless) = &spec.unless {
            validate_query(name, unless)?;
        }
    }
    Ok(())
}

fn validate_query(name: &str, source: &str) -> Result<()> {
    let lang =
        crate::lang::Lang::from_name(name).with_context(|| format!("unknown language {name:?}"))?;
    tree_sitter::Query::new(&lang.grammar(), source)
        .with_context(|| format!("invalid {name} query"))?;
    Ok(())
}

#[derive(Debug, Default)]
pub struct Catalog {
    pub rules: BTreeMap<String, Rule>,
}

/// The built-in catalog is compiled into the binary so `sf` works in a repo
/// that has never seen this tool. A repo may add its own rules on top.
const BUILTIN: &[(&str, &str)] = &[
    ("L0/exceptions-have-one-home.yaml", include_str!("../catalog/L0/exceptions-have-one-home.yaml")),
    ("L0/persistence-stays-in-repositories.yaml", include_str!("../catalog/L0/persistence-stays-in-repositories.yaml")),
    ("L0/one-entrypoint-per-file.yaml", include_str!("../catalog/L0/one-entrypoint-per-file.yaml")),
    ("L0/no-cross-layer-import.yaml", include_str!("../catalog/L0/no-cross-layer-import.yaml")),
    ("L1/complexity-ceiling.yaml", include_str!("../catalog/L1/complexity-ceiling.yaml")),
    ("L1/no-blanket-suppression.yaml", include_str!("../catalog/L1/no-blanket-suppression.yaml")),
    ("L1/skipped-tests-state-a-reason.yaml", include_str!("../catalog/L1/skipped-tests-state-a-reason.yaml")),
    ("L1/no-untyped-escape-hatch.yaml", include_str!("../catalog/L1/no-untyped-escape-hatch.yaml")),
    ("L2/generated-files-are-locked.yaml", include_str!("../catalog/L2/generated-files-are-locked.yaml")),
    ("L2/dependencies-change-deliberately.yaml", include_str!("../catalog/L2/dependencies-change-deliberately.yaml")),
    ("L2/no-permanent-exception.yaml", include_str!("../catalog/L2/no-permanent-exception.yaml")),
    ("L3/gate-has-fresh-evidence.yaml", include_str!("../catalog/L3/gate-has-fresh-evidence.yaml")),
    ("L3/gate-covers-the-plan.yaml", include_str!("../catalog/L3/gate-covers-the-plan.yaml")),
    ("L4/doc-links-resolve.yaml", include_str!("../catalog/L4/doc-links-resolve.yaml")),
    ("L4/root-files-are-declared.yaml", include_str!("../catalog/L4/root-files-are-declared.yaml")),
    ("L4/every-rule-has-a-why.yaml", include_str!("../catalog/L4/every-rule-has-a-why.yaml")),
    ("L4/plan-declares-exit-condition.yaml", include_str!("../catalog/L4/plan-declares-exit-condition.yaml")),
    ("L4/plan-criterion-names-its-check.yaml", include_str!("../catalog/L4/plan-criterion-names-its-check.yaml")),
    ("L4/claim-cites-its-evidence.yaml", include_str!("../catalog/L4/claim-cites-its-evidence.yaml")),
    ("L4/rule-prose-names-a-real-command.yaml", include_str!("../catalog/L4/rule-prose-names-a-real-command.yaml")),
    ("L5/every-check-has-a-mutation-test.yaml", include_str!("../catalog/L5/every-check-has-a-mutation-test.yaml")),
    ("L5/no-inert-rule.yaml", include_str!("../catalog/L5/no-inert-rule.yaml")),
    ("L2/factory-config-is-locked.yaml", include_str!("../catalog/L2/factory-config-is-locked.yaml")),
    ("L2/policy-only-tightens.yaml", include_str!("../catalog/L2/policy-only-tightens.yaml")),
    ("L2/derived-artifacts-match-their-source.yaml", include_str!("../catalog/L2/derived-artifacts-match-their-source.yaml")),
    ("L6/dependency-vulnerabilities-are-scanned.yaml", include_str!("../catalog/L6/dependency-vulnerabilities-are-scanned.yaml")),
    ("L6/secrets-are-scanned.yaml", include_str!("../catalog/L6/secrets-are-scanned.yaml")),
    ("L6/insecure-patterns-are-scanned.yaml", include_str!("../catalog/L6/insecure-patterns-are-scanned.yaml")),
    ("L6/dead-code-is-detected.yaml", include_str!("../catalog/L6/dead-code-is-detected.yaml")),
    ("L6/data-races-are-detected.yaml", include_str!("../catalog/L6/data-races-are-detected.yaml")),
    ("L6/performance-regression-is-guarded.yaml", include_str!("../catalog/L6/performance-regression-is-guarded.yaml")),
    ("L6/no-blocking-call-while-holding-a-lock.yaml", include_str!("../catalog/L6/no-blocking-call-while-holding-a-lock.yaml")),
    ("L6/one-lock-at-a-time.yaml", include_str!("../catalog/L6/one-lock-at-a-time.yaml")),
    ("L6/workflows-are-scanned.yaml", include_str!("../catalog/L6/workflows-are-scanned.yaml")),
];

impl Catalog {
    pub fn builtin() -> Result<Catalog> {
        let mut catalog = Catalog::default();
        for (path, body) in BUILTIN {
            let rule: Rule = serde_yaml::from_str(body)
                .with_context(|| format!("built-in catalog rule {path} is malformed"))?;
            catalog.insert(rule, path)?;
        }
        Ok(catalog)
    }

    /// Load repo-local rules from `<root>/.software-factory/rules/**.yaml`.
    pub fn extend_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        let mut entries: Vec<_> = walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && matches!(e.path().extension().and_then(|s| s.to_str()), Some("yaml" | "yml"))
            })
            .map(|e| e.into_path())
            .collect();
        entries.sort();
        for path in entries {
            let body = std::fs::read_to_string(&path)?;
            let rule: Rule = serde_yaml::from_str(&body)
                .with_context(|| format!("local rule {} is malformed", path.display()))?;
            self.insert(rule, &path.display().to_string())?;
        }
        Ok(())
    }

    fn insert(&mut self, rule: Rule, origin: &str) -> Result<()> {
        if rule.why.trim().is_empty() || rule.fix.trim().is_empty() {
            bail!("rule {} ({origin}) must carry both `why` and `fix`", rule.id);
        }
        rule.validate().with_context(|| format!("rule {} ({origin}) is not runnable", rule.id))?;
        if let Some(previous) = self.rules.insert(rule.id.clone(), rule) {
            bail!("duplicate rule id {} (second definition at {origin})", previous.id);
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Rule> {
        self.rules.get(id)
    }
}

#[cfg(test)]
mod completeness {
    use super::BUILTIN;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    fn catalog_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("catalog")
    }

    fn on_disk() -> BTreeSet<String> {
        walkdir::WalkDir::new(catalog_dir())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path().extension().and_then(|s| s.to_str()) == Some("yaml")
            })
            .map(|e| {
                e.path()
                    .strip_prefix(catalog_dir())
                    .expect("walked entry is under catalog_dir")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    /// `BUILTIN` is what `sf` ships without a checkout. A `catalog/**/*.yaml`
    /// with no entry here never reaches a fresh install; an entry with no
    /// file already failed to compile. Both directions, offenders named.
    #[test]
    fn builtin_matches_catalog_dir() {
        let registered: BTreeSet<String> = BUILTIN.iter().map(|(path, _)| path.to_string()).collect();
        let disk = on_disk();

        let unregistered: Vec<_> = disk.difference(&registered).collect();
        let missing: Vec<_> = registered.difference(&disk).collect();

        assert!(
            unregistered.is_empty() && missing.is_empty(),
            "catalog::BUILTIN is out of sync with catalog/:\n\
             files on disk with no BUILTIN entry: {unregistered:?}\n\
             BUILTIN entries with no file on disk: {missing:?}"
        );
    }
}
