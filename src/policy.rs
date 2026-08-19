//! Per-repo policy: which rules are on, and how this repo's paths map onto
//! the catalog's neutral vocabulary.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const POLICY_PATH: &str = ".software-factory/policy.yaml";
pub const RATCHET_PATH: &str = ".software-factory/ratchet.yaml";
pub const RULES_DIR: &str = ".software-factory/rules";
pub const FIXTURES_DIR: &str = ".software-factory/mutations";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Options {
    // shape
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_live_in: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_not_live_in: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_per_file: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name_suffix: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_regex: Option<String>,
    // complexity
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
    // text pattern
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbidden: Vec<TextPattern>,
    // scope
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    // lock
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock_file: Option<String>,
    // cadence
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowlist_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_order: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    // evidence
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbidden_in_goal: Vec<String>,
    // toolchain: language -> any one of these tool invocations must appear
    // somewhere the repository actually runs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<String, Vec<String>>,
    // policy tightening: where to read the previous policy from when git
    // history is not available (fixtures, shallow checkouts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
    /// How many `inner` matches inside one `outer` are tolerated. The default
    /// of 1 means "any occurrence is a finding"; 2 expresses "a second one in
    /// the same scope is the hazard", which is how nested locking is stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_inner: Option<usize>,
    /// The command a `command` rule runs, from the repository root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// Skip lines that are entirely a comment. Opt-in, because a rule about
    /// suppression comments needs exactly the opposite.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_comment_lines: bool,
}

/// Shallow merge: keys present in `over` win, everything else falls through to
/// the catalog default. One level is deliberate — a rule option that needs
/// deep merging is a rule option that should have been two options.
pub fn merge(base: &serde_yaml::Value, over: &serde_yaml::Value) -> serde_yaml::Value {
    match (base, over) {
        (serde_yaml::Value::Mapping(b), serde_yaml::Value::Mapping(o)) => {
            let mut out = b.clone();
            for (k, v) in o {
                out.insert(k.clone(), v.clone());
            }
            serde_yaml::Value::Mapping(out)
        }
        (_, serde_yaml::Value::Null) => base.clone(),
        _ => over.clone(),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextPattern {
    /// The shape that is not acceptable.
    pub regex: String,
    /// The shape that is. A line matching `regex` but also `unless` passes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unless: Option<String>,
    /// Shown verbatim on failure. This is the agent-facing documentation:
    /// it must name the alternative, not just the ban.
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleSetting {
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Kept as raw YAML so it can be merged over the catalog rule's
    /// `defaults` without "absent" and "empty" collapsing into each other.
    #[serde(default)]
    pub options: serde_yaml::Value,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Gate {
    /// Touching any of these paths activates the gate. Activation is by
    /// changed paths, never by a label or a sentence in a pull request.
    pub activation: Vec<String>,
    /// Where the evidence manifest for this gate lives.
    pub evidence: String,
    /// The plan whose acceptance criteria this gate is supposed to enforce.
    /// `L3.GATE_COVERS_THE_PLAN` reads the criteria there and requires every
    /// check they name to appear in `required_assertions` below.
    #[serde(default)]
    pub plan: Option<String>,
    /// Assertions every run of this gate must carry, declared here rather than
    /// only in the manifest. A manifest states what it owed, so a run that
    /// under-declares its own obligations passes; policy lives outside the
    /// candidate implementation and is not editable by the change under
    /// review. The two lists are unioned — a manifest may add, never subtract.
    #[serde(default)]
    pub required_assertions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Policy {
    pub version: u32,
    pub project: Project,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleSetting>,
    #[serde(default)]
    pub gates: BTreeMap<String, Gate>,
    #[serde(default)]
    pub docs: Docs,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    /// Languages this repo asks to be parsed. A rule with no query for a
    /// listed language simply does not apply to it.
    pub languages: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Other checkouts this policy governs, as paths relative to the root —
    /// usually symlinks to sibling repositories. Symlinks are followed only
    /// here, never during the ordinary walk, because a package manager's
    /// symlink farm would otherwise be walked as source.
    ///
    /// Findings keep the declared prefix, so a rule reads the same whether the
    /// checkout lives beside this one or somewhere else entirely.
    #[serde(default)]
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Docs {
    #[serde(default)]
    pub scan: Vec<String>,
    #[serde(default)]
    pub plans_dir: Option<String>,
    /// Where the generated rule reference lives. Repositories that already
    /// have a documentation convention should not be made to grow a `docs/`
    /// directory just to satisfy this tool.
    #[serde(default)]
    pub rules_document: Option<String>,
}

impl Docs {
    pub fn rules_document(&self) -> &str {
        self.rules_document.as_deref().unwrap_or("docs/rules.md")
    }
}

impl Policy {
    pub fn load(root: &Path) -> Result<Policy> {
        let path = root.join(POLICY_PATH);
        let body = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "no policy at {} — run `sf init` in this repository first",
                path.display()
            )
        })?;
        let policy: Policy = serde_yaml::from_str(&body)
            .with_context(|| format!("{} is malformed", path.display()))?;
        anyhow::ensure!(policy.version == 1, "unsupported policy version");
        Ok(policy)
    }

    /// Enabled entries as (instance id, catalog rule id).
    ///
    /// A monorepo needs the same rule twice with different settings — a
    /// complexity ceiling of 12 in the new packages and 20 in the one nobody
    /// has had time to split up. Writing the key as `RULE@name` gives that
    /// instance its own options, its own findings and its own ratchet entries,
    /// while still resolving to one catalog rule with one written reason.
    pub fn instances(&self) -> Vec<(String, String)> {
        self.rules
            .iter()
            .filter(|(_, setting)| setting.enabled)
            .map(|(key, _)| (key.clone(), base_rule_id(key).to_string()))
            .collect()
    }

    /// Is any instance of this catalog rule enabled?
    pub fn any_instance_enabled(&self, rule_id: &str) -> bool {
        self.rules
            .iter()
            .any(|(key, setting)| setting.enabled && base_rule_id(key) == rule_id)
    }
}

/// `L1.COMPLEXITY_CEILING@legacy` names the `L1.COMPLEXITY_CEILING` rule.
pub fn base_rule_id(key: &str) -> &str {
    key.split('@').next().unwrap_or(key)
}

/// The default noise every repo wants skipped, before policy excludes apply.
pub const ALWAYS_SKIP: &[&str] = &[
    ".git", "node_modules", "target", "dist", "build", ".venv", "venv",
    "__pycache__", ".mypy_cache", ".ruff_cache", ".pytest_cache", "vendor",
    ".next", ".turbo", "coverage", ".software-factory/mutations",
];

pub fn repo_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.canonicalize()?;
    loop {
        if current.join(POLICY_PATH).exists() || current.join(".git").exists() {
            return Ok(current);
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => anyhow::bail!("not inside a git repository"),
        }
    }
}
