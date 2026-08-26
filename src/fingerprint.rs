//! What a repository agreed to when it adopted a version of the catalog.
//!
//! The catalog ships inside the binary. A consumer's `policy.yaml` names rule
//! ids and nothing else, so upgrading `sf` can change what an already-enabled
//! rule matches with no diff in the consuming repository at all. A new rule is
//! safe by construction — `Policy::instances` only iterates what the policy
//! lists, so a rule nobody enabled never runs — but a rule that keeps its id
//! and changes its reach arrives silently, and the direction that matters is
//! the quiet one: a rule that got weaker turns a red repository green without
//! anybody deciding that.
//!
//! This module records the reach of every enabled rule at lock time, so the
//! next binary can be compared against it. `Reach` deliberately reduces a rule
//! to a handful of countable dimensions, the same discipline
//! `checks::tightening::option_size` already applies to a policy diff: what
//! cannot be compared honestly is not compared at all.

use crate::catalog::{Catalog, CheckKind, Rule};
use crate::digest;
use crate::finding::Severity;
use crate::policy::{Options, Policy};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const CATALOG_LOCK_PATH: &str = ".software-factory/catalog.lock.json";
const SCHEMA_VERSION: u32 = 1;

/// The countable reach of one rule. Every field is a number or an ordinal
/// precisely so that "did this get weaker" has an answer a check can give.
/// Prose is absent on purpose: a rewritten `why` is not a behaviour change,
/// and treating it as one would train everybody to re-lock on noise.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Reach {
    pub severity: Severity,
    /// Glob counts. `scope` shrinking means the rule reads fewer files.
    #[serde(default)]
    pub scope: usize,
    #[serde(default)]
    pub exclude: usize,
    /// A ceiling. Raised means more code is now acceptable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
    /// `text_pattern` entries. Fewer patterns is a smaller ban.
    #[serde(default)]
    pub forbidden: usize,
    /// Forbidden entries carrying a `scope`, `exclude` or `unless` — each one
    /// is a pattern that stopped applying somewhere it used to apply. This is
    /// the dimension that makes a per-pattern narrowing visible; without it,
    /// restricting a pattern to one language reads as no change at all,
    /// because the pattern count is untouched.
    #[serde(default)]
    pub narrowed_patterns: usize,
    /// Per-language query maps for `shape` and `nested`. A language dropped is
    /// a language the rule stopped covering.
    #[serde(default)]
    pub languages: usize,
    /// `toolchain` entries. A language dropped from `tools` makes the rule
    /// permanently silent for a project that declares it.
    #[serde(default)]
    pub tools: usize,
    /// Placement. More allowed homes is looser; fewer forbidden homes is looser.
    #[serde(default)]
    pub must_live_in: usize,
    #[serde(default)]
    pub must_not_live_in: usize,
}

impl Reach {
    pub fn of(rule: &Rule) -> Reach {
        let options: Options = serde_yaml::from_value(rule.defaults.clone()).unwrap_or_default();
        Reach {
            severity: rule.severity,
            scope: options.scope.len(),
            exclude: options.exclude.len(),
            max: options.max,
            forbidden: options.forbidden.len(),
            narrowed_patterns: options
                .forbidden
                .iter()
                .filter(|p| !p.scope.is_empty() || !p.exclude.is_empty() || p.unless.is_some())
                .count(),
            languages: match &rule.check {
                CheckKind::Shape { languages } => languages.len(),
                CheckKind::Nested { languages } => languages.len(),
                _ => 0,
            },
            tools: options.tools.len(),
            must_live_in: options.must_live_in.len(),
            must_not_live_in: options.must_not_live_in.len(),
        }
    }

    /// Every way this reach is smaller than `previous`, in the consumer's
    /// words rather than the field's. An empty vector means the rule is at
    /// least as strong as it was, which is the case that passes silently.
    pub fn weakenings(&self, previous: &Reach) -> Vec<String> {
        let mut found = Vec::new();
        if rank(self.severity) < rank(previous.severity) {
            found.push(format!("severity dropped from {} to {}", previous.severity, self.severity));
        }
        // A ceiling is the one dimension that is optional, so it cannot join
        // the table below: absent in either version means this model has
        // nothing to say, which is different from a count of zero.
        if let (Some(now), Some(was)) = (self.max, previous.max)
            && now > was
        {
            found.push(format!("ceiling raised from {was} to {now}"));
        }
        for dimension in COUNTED {
            if let Some(message) = dimension.weakening(self, previous) {
                found.push(message);
            }
        }
        found
    }
}

/// Which direction of movement makes a rule weaker.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Weaker {
    /// More of it means the rule sees less: exclusions, per-pattern
    /// restrictions, permitted homes.
    WhenItGrows,
    /// Less of it means the rule sees less: scope globs, forbidden patterns,
    /// languages, tools, forbidden locations.
    WhenItShrinks,
}

/// One countable dimension of `Reach`. A table rather than ten near-identical
/// `if` blocks: the ten comparisons differ only in a field, a direction and a
/// sentence, and writing that out ten times is how the eleventh gets the
/// direction backwards without anybody noticing.
struct Counted {
    get: fn(&Reach) -> usize,
    weaker: Weaker,
    /// `(previous, now)` to the sentence a person reads when deciding whether
    /// to accept the loosening.
    say: fn(usize, usize) -> String,
    /// Skip when the previous value was zero. A scope of zero already means
    /// "everywhere", so going from zero to a list is a narrowing the count
    /// alone reads backwards.
    needs_previous: bool,
}

impl Counted {
    fn weakening(&self, now: &Reach, previous: &Reach) -> Option<String> {
        let (was, is) = ((self.get)(previous), (self.get)(now));
        if self.needs_previous && was == 0 {
            return None;
        }
        let weakened = match self.weaker {
            Weaker::WhenItGrows => is > was,
            Weaker::WhenItShrinks => is < was,
        };
        weakened.then(|| (self.say)(was, is))
    }
}

const COUNTED: &[Counted] = &[
    Counted {
        get: |r| r.exclude,
        weaker: Weaker::WhenItGrows,
        say: |was, is| format!("gained {} exclusion(s), {was} to {is}", is - was),
        needs_previous: false,
    },
    Counted {
        get: |r| r.scope,
        weaker: Weaker::WhenItShrinks,
        say: |was, is| format!("scope shrank from {was} to {is} glob(s)"),
        needs_previous: true,
    },
    Counted {
        get: |r| r.forbidden,
        weaker: Weaker::WhenItShrinks,
        say: |was, is| format!("dropped {} forbidden pattern(s), {was} to {is}", was - is),
        needs_previous: false,
    },
    Counted {
        get: |r| r.narrowed_patterns,
        weaker: Weaker::WhenItGrows,
        say: |was, is| {
            format!(
                "{} more forbidden pattern(s) now restricted by scope, exclude or unless, {was} to {is}",
                is - was
            )
        },
        needs_previous: false,
    },
    Counted {
        get: |r| r.languages,
        weaker: Weaker::WhenItShrinks,
        say: |was, is| format!("stopped covering {} language(s), {was} to {is}", was - is),
        needs_previous: false,
    },
    Counted {
        get: |r| r.tools,
        weaker: Weaker::WhenItShrinks,
        say: |was, is| {
            format!("dropped {} language(s) from its tools map, {was} to {is}", was - is)
        },
        needs_previous: false,
    },
    Counted {
        get: |r| r.must_live_in,
        weaker: Weaker::WhenItGrows,
        say: |was, is| format!("allowed {} more home(s), {was} to {is}", is - was),
        needs_previous: false,
    },
    Counted {
        get: |r| r.must_not_live_in,
        weaker: Weaker::WhenItShrinks,
        say: |was, is| format!("forbids {} fewer location(s), {was} to {is}", was - is),
        needs_previous: true,
    },
];

/// `Severity` is declared most-severe-first, so its discriminant runs the
/// wrong way for a comparison about strength.
fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Low => 0,
        Severity::Medium => 1,
        Severity::High => 2,
        Severity::Critical => 3,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogLock {
    pub schema_version: u32,
    /// Which `sf` wrote this. Not compared by any check: it is here so that a
    /// person reading a red build knows which upgrade to look at.
    pub sf_version: String,
    /// Digest of the whole embedded catalog. A cheap "did anything at all
    /// move" that does not depend on the reach model being complete.
    pub catalog_digest: String,
    pub rules: BTreeMap<String, Reach>,
}

impl CatalogLock {
    pub fn of(catalog: &Catalog, policy: &Policy) -> CatalogLock {
        let rules = catalog
            .rules
            .iter()
            .filter(|(id, _)| policy.any_instance_enabled(id))
            .map(|(id, rule)| (id.clone(), Reach::of(rule)))
            .collect();
        CatalogLock {
            schema_version: SCHEMA_VERSION,
            sf_version: env!("CARGO_PKG_VERSION").to_string(),
            catalog_digest: catalog_digest(),
            rules,
        }
    }

    pub fn load(root: &Path) -> Result<Option<CatalogLock>> {
        let path = root.join(CATALOG_LOCK_PATH);
        if !path.exists() {
            return Ok(None);
        }
        let body = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {CATALOG_LOCK_PATH}"))?;
        let lock: CatalogLock = serde_json::from_str(&body)
            .with_context(|| format!("{CATALOG_LOCK_PATH} is not a catalog lock this sf understands"))?;
        anyhow::ensure!(
            lock.schema_version == SCHEMA_VERSION,
            "{CATALOG_LOCK_PATH} declares schema_version {}, and this sf writes {SCHEMA_VERSION}",
            lock.schema_version
        );
        Ok(Some(lock))
    }

    pub fn write(&self, root: &Path) -> Result<String> {
        let path = root.join(CATALOG_LOCK_PATH);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(self)?))?;
        Ok(CATALOG_LOCK_PATH.to_string())
    }
}

/// Digest of the catalog compiled into this binary, as a set: the same rules
/// in a different registry order produce the same digest, because the order of
/// `BUILTIN` is not something a consumer agreed to.
pub fn catalog_digest() -> String {
    let mut entries: Vec<(String, String)> = crate::catalog::BUILTIN
        .iter()
        .map(|(path, body)| (path.to_string(), digest::hex(body.as_bytes())))
        .collect();
    digest::tree(&mut entries)
}

/// What `sf --version` prints. The catalog digest belongs here because the
/// version number alone does not identify the rules: two builds of the same
/// version from different commits enforce different things, and the consumer
/// who has to reproduce a finding needs the second half.
pub fn version_line() -> &'static str {
    // `clap` wants a `&'static str`, and the digest is only knowable at run
    // time. Computed once and kept, rather than leaked per call.
    static LINE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    LINE.get_or_init(|| {
        let digest = catalog_digest();
        format!(
            "{} (catalog {}, {} rules)",
            env!("CARGO_PKG_VERSION"),
            &digest[..12],
            crate::catalog::BUILTIN.len()
        )
    })
    .as_str()
}

/// One test per dimension of `Reach`, because `sf verify` proves only that the
/// rule's fixture produces at least one finding. A fixture carrying all ten
/// would let nine broken dimensions hide behind the one that still works,
/// which is the same weakness this repository already met in
/// `L5.NO_INERT_RULE`: a fixture with several mutations is not a per-mutation
/// proof.
#[cfg(test)]
mod direction {
    use super::*;

    fn base() -> Reach {
        Reach {
            severity: Severity::High,
            scope: 3,
            exclude: 1,
            max: Some(10),
            forbidden: 4,
            narrowed_patterns: 1,
            languages: 5,
            tools: 4,
            must_live_in: 1,
            must_not_live_in: 2,
        }
    }

    /// Every field, mutated in the weakening direction, one at a time. The
    /// message is asserted on because it is what a person reads at the moment
    /// they have to decide whether to accept the loosening.
    #[test]
    fn each_dimension_is_detected_alone() {
        let cases: Vec<(&str, Reach, &str)> = vec![
            ("severity", Reach { severity: Severity::Low, ..base() }, "severity dropped"),
            ("exclude", Reach { exclude: 3, ..base() }, "gained 2 exclusion(s)"),
            ("scope", Reach { scope: 1, ..base() }, "scope shrank from 3 to 1"),
            ("max", Reach { max: Some(20), ..base() }, "ceiling raised from 10 to 20"),
            ("forbidden", Reach { forbidden: 2, ..base() }, "dropped 2 forbidden pattern(s)"),
            (
                "narrowed_patterns",
                Reach { narrowed_patterns: 3, ..base() },
                "2 more forbidden pattern(s) now restricted",
            ),
            ("languages", Reach { languages: 4, ..base() }, "stopped covering 1 language(s)"),
            ("tools", Reach { tools: 1, ..base() }, "dropped 3 language(s) from its tools map"),
            ("must_live_in", Reach { must_live_in: 4, ..base() }, "allowed 3 more home(s)"),
            (
                "must_not_live_in",
                Reach { must_not_live_in: 1, ..base() },
                "forbids 1 fewer location(s)",
            ),
        ];
        for (name, weaker, expected) in cases {
            let found = weaker.weakenings(&base());
            assert_eq!(
                found.len(),
                1,
                "{name} alone should read as exactly one weakening, got {found:?}"
            );
            assert!(
                found[0].contains(expected),
                "the {name} message must say what a person has to decide about: {}",
                found[0]
            );
        }
    }

    /// The direction that must stay silent. A rule that got stronger costs a
    /// consumer a red build they can see and fix; taxing it would train
    /// everybody to re-lock reflexively, which is how the fingerprint stops
    /// meaning anything.
    #[test]
    fn tightening_is_silent() {
        let stronger = Reach {
            severity: Severity::Critical,
            scope: 6,
            exclude: 0,
            max: Some(8),
            forbidden: 6,
            narrowed_patterns: 0,
            languages: 6,
            tools: 5,
            must_live_in: 0,
            must_not_live_in: 4,
        };
        assert!(
            stronger.weakenings(&base()).is_empty(),
            "a strictly stronger rule must not be reported: {:?}",
            stronger.weakenings(&base())
        );
        assert!(base().weakenings(&base()).is_empty(), "an unchanged rule must be silent");
    }

    /// A rule with no ceiling in either version, and a rule that gained one,
    /// must not read as a raise. `Option` comparison is where an off-by-one
    /// direction bug would hide, because `None` sorts below `Some`.
    #[test]
    fn an_absent_ceiling_is_not_a_raised_one() {
        let no_max = Reach { max: None, ..base() };
        assert!(no_max.weakenings(&Reach { max: None, ..base() }).is_empty());
        assert!(
            Reach { max: Some(4), ..base() }.weakenings(&no_max).is_empty(),
            "gaining a ceiling is a tightening"
        );
        assert!(
            no_max.weakenings(&base()).is_empty(),
            "losing the ceiling field is not something this model claims to judge"
        );
    }

    /// The catalog digest has to be stable across runs of the same binary and
    /// independent of registry order, or every `sf lock` would rewrite it.
    #[test]
    fn the_catalog_digest_is_stable() {
        assert_eq!(catalog_digest(), catalog_digest());
        assert_eq!(catalog_digest().len(), 64, "sha-256 in hex");
    }

    /// `Reach::of` reads `defaults` with `unwrap_or_default()`, so a rule whose
    /// defaults stopped matching the option schema would fingerprint as all
    /// zeroes instead of failing. All zeroes reads as "reaches nothing", which
    /// is exactly the state in which a later loosening produces no finding.
    /// `Rule::validate` already rejects such a rule at load; this asserts the
    /// two agree, so the fingerprint can never be quietly built from a parse
    /// that failed.
    #[test]
    fn no_shipped_rule_fingerprints_from_a_failed_parse() {
        let catalog = Catalog::builtin().expect("the shipped catalog loads");
        for (id, rule) in &catalog.rules {
            serde_yaml::from_value::<Options>(rule.defaults.clone())
                .unwrap_or_else(|e| panic!("{id} defaults do not parse into Options: {e}"));
        }
    }
}
