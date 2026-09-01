//! Per-repo policy: which rules are on, and how this repo's paths map onto
//! the catalog's neutral vocabulary.

use crate::manifest::{Declared, Version};
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
    /// Words that name nothing in particular when they appear as a run's
    /// `actor`. The field records what performed the run, and the whole
    /// point of L3 is that the actor is shaped like the customer, so a
    /// manifest crediting the run to "scripted" has recorded a replay, not
    /// a proof.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbidden_actors: Vec<String>,
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
    /// Restrict *this pattern* to a subset of the rule's own `scope` — the
    /// same glob vocabulary, one level down. A rule with several spellings
    /// of one concept, one per language, must not point language B's
    /// spelling at language A's files: `:\s*any\b` is TypeScript's `any`,
    /// but it also matches Ruby's `:any?` symbol wherever both live under
    /// the same rule's scope. Empty means "wherever the rule's scope
    /// already reaches", so a rule with one flat pattern list — the common
    /// case — is unaffected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleSetting {
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Kept as raw YAML so it can be merged over the catalog rule's
    /// `defaults` without "absent" and "empty" collapsing into each other.
    #[serde(default)]
    pub options: serde_yaml::Value,
    /// The dependency version this instance is about, if it is only about
    /// one. See [`When`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

/// A rule instance that is only correct while the dependency it describes is
/// pinned to the version it was written for.
///
/// ```yaml
/// PACK_RULE_ID@tailwind3:
///   enabled: true
///   when:
///     dependency: tailwindcss
///     manifest: package.json
///     version: "^3"
/// ```
///
/// A deprecation rule for Tailwind 3 is only correct while 3 is what is
/// installed. Once the pin moves, its patterns describe an API nobody calls
/// and it reports green forever, which reads exactly like a rule that is
/// protecting you.
///
/// The condition is read from the manifest, which is inside the scope of
/// `L2.DEPENDENCIES_CHANGE_DELIBERATELY`: the input cannot move without a
/// lock update in the same commit, so `when` is gated by a rule that already
/// exists rather than being a new thing to trust.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct When {
    /// The package this rule is about, spelled as the manifest spells it.
    pub dependency: String,
    /// Which manifest declares it, relative to the repository root.
    pub manifest: String,
    /// The range this instance was written for: `^3`, `~7.1`, `>=5`, `3.4`.
    pub version: String,
}

/// Whether a rule instance's `when` still describes this repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activation {
    /// No condition, or a condition the manifest satisfies.
    Active,
    /// The condition went false. The rule does not run — it is about a
    /// version this repository does not have — and it does not go quiet
    /// either: `L5.NO_INERT_RULE` reports this reason. Silently disabling
    /// itself is how a policy becomes decoration, and it would hand an agent
    /// a way to switch a rule off by editing a dependency.
    Stale(String),
}

impl Activation {
    pub fn stale_reason(&self) -> Option<&str> {
        match self {
            Activation::Active => None,
            Activation::Stale(reason) => Some(reason),
        }
    }
}

/// Decide one instance's `when` against the manifest on disk.
pub fn activation(root: &Path, setting: &RuleSetting) -> Result<Activation> {
    let Some(when) = &setting.when else {
        return Ok(Activation::Active);
    };
    let (dependency, manifest) = (&when.dependency, &when.manifest);
    let stale = match crate::manifest::declared(root, manifest, dependency)? {
        Declared::NoManifest => format!("its `when` reads `{manifest}`, which is not in this repository"),
        Declared::UnknownFormat => format!(
            "its `when` reads `{manifest}`, which is not a manifest this binary knows how to read ({})",
            crate::manifest::READABLE
        ),
        Declared::Malformed(error) => {
            format!("its `when` reads `{manifest}`, which did not parse: {error}")
        }
        Declared::Absent => format!(
            "its `when` is about {dependency}, which `{manifest}` does not declare — a rule about a dependency no manifest declares is a rule about somebody else's resolution"
        ),
        Declared::Range(range) => match unsatisfied(&when.version, &range) {
            Some(detail) => format!("its `when` is about {dependency} {}, and `{manifest}` {detail}", when.version),
            None => return Ok(Activation::Active),
        },
    };
    Ok(Activation::Stale(stale))
}

/// Why the declared range does not answer the expected one, if it does not.
fn unsatisfied(expected: &str, declared: &str) -> Option<String> {
    let Some(found) = Version::parse(declared) else {
        return Some(match declared.trim().is_empty() {
            true => "declares it with no version at all".to_string(),
            false => format!("declares `{declared}`, which names no version to compare"),
        });
    };
    if satisfies(expected, &found) {
        return None;
    }
    Some(format!("declares `{declared}`"))
}

/// Does the version a manifest declares fall in the range a `when` names?
///
/// A deliberately small grammar — `^`, `~`, `>=`, `>`, `<=`, `<`, and a bare
/// series like `3` or `3.4` — over the release numbers, and nothing else. A
/// `when` asks whether the pin is still in the series the rule was written
/// for. Answering that does not need pre-release ordering, and a rule
/// activated by a pre-release tag would be a rule about a nightly build.
pub fn satisfies(expected: &str, found: &Version) -> bool {
    let trimmed = expected.trim();
    let (operator, rest) = ["<=", ">=", "~>", "^", "~", "<", ">", "="]
        .iter()
        .find_map(|op| trimmed.strip_prefix(op).map(|rest| (*op, rest)))
        .unwrap_or(("", trimmed));
    let Some(base) = Version::parse(rest) else {
        return false;
    };
    match operator {
        ">=" => *found >= base,
        ">" => *found > base,
        "<=" => *found <= base,
        "<" => *found < base,
        "^" => *found >= base && shares_prefix(found, &base, significant(&base) + 1),
        "~" | "~>" => *found >= base && shares_prefix(found, &base, base.0.len().min(2)),
        // A bare `3` or `3.4` is the series itself: every component someone
        // wrote has to match, and the ones they left off are free.
        _ => shares_prefix(found, &base, base.0.len()),
    }
}

/// The index of the leading component that decides compatibility. Caret on
/// `0.x` locks the minor, which is what every ecosystem that has the operator
/// means by it.
fn significant(base: &Version) -> usize {
    (0..base.0.len()).find(|index| base.at(*index) != 0).unwrap_or(0)
}

fn shares_prefix(found: &Version, base: &Version, width: usize) -> bool {
    (0..width).all(|index| found.at(index) == base.at(index))
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

    /// Whether one instance's `when` still describes this repository. An
    /// instance with no `when` — every instance, until a policy writes one —
    /// is always active.
    pub fn activation_of(&self, root: &Path, instance: &str) -> Result<Activation> {
        match self.rules.get(instance) {
            Some(setting) => activation(root, setting),
            None => Ok(Activation::Active),
        }
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

/// `when:` — parsing it, and the decision it makes about whether an instance
/// runs. The fixture these read against is
/// `.software-factory/mutations/L5.NO_INERT_RULE/`, materialized by
/// `sf fixtures` from `src/fixtures.rs`: a mini-repo whose `package.json`
/// pins `tailwindcss ^4.0.2` and which enables three conditional instances of
/// one rule — `@tailwind3` (the pin moved), `@quickbooks` (a package the
/// manifest never declared) and `@tailwind4` (the condition still holds).
#[cfg(test)]
mod when_conditions {
    use super::*;
    use crate::catalog::Catalog;
    use crate::checks::Ctx;
    use crate::ratchet::Ratchet;
    use crate::scan;

    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES_DIR).join("L5.NO_INERT_RULE")
    }

    fn setting(yaml: &str) -> RuleSetting {
        serde_yaml::from_str(yaml).expect("the rule setting parses")
    }

    #[test]
    fn a_when_parses_into_the_three_fields_it_needs() {
        let parsed = setting(
            "enabled: true\nwhen:\n  dependency: tailwindcss\n  manifest: package.json\n  version: \"^3\"\n",
        );
        let when = parsed.when.expect("a `when` block parses");
        assert_eq!(when.dependency, "tailwindcss");
        assert_eq!(when.manifest, "package.json");
        assert_eq!(when.version, "^3");
        assert!(setting("enabled: true\n").when.is_none(), "a rule without one is unconditional");
    }

    /// The decision itself: the instance whose pin still matches runs and
    /// reports, and the two whose conditions went false do not — while the
    /// same rule, on the same line of the same file, is what all three are.
    #[test]
    fn the_instance_whose_pin_still_matches_is_the_only_one_that_runs() {
        let root = fixture_root();
        let policy = Policy::load(&root).expect("the fixture policy loads");
        let catalog = Catalog::builtin().expect("the built-in catalog loads");
        let files = scan::walk(&root, &policy).expect("the fixture repo scans");
        let ratchet = Ratchet::default();
        let ctx = Ctx {
            root: &root,
            policy: &policy,
            catalog: &catalog,
            files: &files,
            ratchet: &ratchet,
            changed: None,
            base: None,
            today: crate::clock::today(),
            allow_commands: false,
        };
        let findings = crate::checks::run_all(&ctx).expect("the fixture checks run");
        let fired = |instance: &str| findings.iter().any(|f| f.rule == instance);
        assert!(
            fired("L1.NO_BLANKET_SUPPRESSION@tailwind4"),
            "the instance whose `when` the manifest satisfies must still run"
        );
        assert!(
            !fired("L1.NO_BLANKET_SUPPRESSION@tailwind3"),
            "an instance written for a pin that moved must not report on code that is now right"
        );
        assert!(
            !fired("L1.NO_BLANKET_SUPPRESSION@quickbooks"),
            "an instance about a dependency nothing declares must not report"
        );
    }

    /// Not running is only half of it. A condition that went false has to say
    /// so, or `when` is a way to switch a rule off by editing a manifest.
    #[test]
    fn a_condition_that_went_false_names_the_range_and_the_version_found() {
        let root = fixture_root();
        let policy = Policy::load(&root).expect("the fixture policy loads");
        let moved = policy
            .activation_of(&root, "L1.NO_BLANKET_SUPPRESSION@tailwind3")
            .expect("the condition is decidable");
        let reason = moved.stale_reason().expect("a pin that moved is stale");
        assert!(reason.contains("^3"), "names the range it was written for: {reason}");
        assert!(reason.contains("^4.0.2"), "names the version found: {reason}");

        let undeclared = policy
            .activation_of(&root, "L1.NO_BLANKET_SUPPRESSION@quickbooks")
            .expect("the condition is decidable");
        let reason = undeclared.stale_reason().expect("an undeclared dependency is stale");
        assert!(reason.contains("node-quickbooks"), "names the package: {reason}");
        assert!(reason.contains("does not declare"), "says what is missing: {reason}");

        let holds = policy
            .activation_of(&root, "L1.NO_BLANKET_SUPPRESSION@tailwind4")
            .expect("the condition is decidable");
        assert_eq!(holds, Activation::Active);
    }

    /// A `when` this tool cannot decide is a finding too. Silence here would
    /// be the same hole in a different shape: the rule stops running and
    /// nothing says why.
    #[test]
    fn a_manifest_that_is_missing_or_unreadable_is_reported_rather_than_assumed() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let conditional = |manifest: &str, version: &str| {
            setting(&format!(
                "enabled: true\nwhen:\n  dependency: serde\n  manifest: {manifest}\n  version: \"{version}\"\n"
            ))
        };
        let missing = activation(root, &conditional("package.json", "^1"))
            .expect("the condition is decidable");
        let reason = missing.stale_reason().expect("no manifest is stale");
        assert!(reason.contains("package.json"), "names the manifest: {reason}");

        // Cargo.lock exists here and is deliberately not a manifest this tool
        // reads: a `when` resolves the range the team wrote, never the version
        // a resolver picked.
        let unreadable =
            activation(root, &conditional("Cargo.lock", "^1")).expect("the condition is decidable");
        let reason = unreadable.stale_reason().expect("an unreadable manifest is stale");
        assert!(reason.contains("Cargo.lock"), "names the file: {reason}");
        assert!(reason.contains("Cargo.toml"), "names what it can read instead: {reason}");

        // And the case that has to keep working: this repository's own
        // manifest, read for a dependency it really declares.
        assert_eq!(
            activation(root, &conditional("Cargo.toml", "^1")).expect("decidable"),
            Activation::Active
        );
        let moved =
            activation(root, &conditional("Cargo.toml", "^2")).expect("the condition is decidable");
        assert!(moved.stale_reason().is_some(), "serde is pinned to 1.x here, not 2.x");
    }

    #[test]
    fn the_range_grammar_answers_the_question_a_when_is_asking() {
        let matches: &[(&str, &str)] = &[
            ("^3", "3.4.1"),
            ("^3.4", "3.9.0"),
            ("^0.110", "0.110.3"),
            ("~1.2", "1.2.9"),
            ("~> 7.1", "7.1.3"),
            (">=5", "6.0.0"),
            ("<4", "3.9.9"),
            ("3", "3.0.0"),
            ("3.4", "3.4.7"),
        ];
        for (expected, found) in matches {
            let found = Version::parse(found).expect("the version parses");
            assert!(satisfies(expected, &found), "{expected} should match {found:?}");
        }
        let misses: &[(&str, &str)] = &[
            ("^3", "4.0.2"),
            ("^3.4", "3.3.9"),
            // Caret on a leading zero locks the minor: 0.110 and 0.111 are
            // not the same series, which is what every ecosystem with the
            // operator means by it.
            ("^0.110", "0.111.0"),
            ("~1.2", "1.3.0"),
            (">=5", "4.9.9"),
            ("<4", "4.0.0"),
            ("3", "4.0.0"),
            ("3.4", "3.5.0"),
        ];
        for (expected, found) in misses {
            let found = Version::parse(found).expect("the version parses");
            assert!(!satisfies(expected, &found), "{expected} should not match {found:?}");
        }
    }
}
