//! A plan written as a declaration: YAML frontmatter carrying what it
//! promises, where each promise comes from, and what it crosses and refuses.
//!
//! The older plan rules read criteria and the exit through [`declared_criteria`]
//! and [`declares_exit`], so a repository can move one plan at a time.
//! [`plan_declaration`] is `L4.PLAN_IS_A_LINKED_DECLARATION`.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::policy::Options;
use crate::scan;
use anyhow::Result;
use regex::Regex;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeSet;

/// The keys a declaration may carry. Closed on purpose: a free key is where
/// the context nobody owns grows back.
pub const DECLARATION_KEYS: &[&str] = &[
    "base",
    "sources",
    "criteria",
    "touches",
    "untouched",
    "crosses",
    "guarantee",
    "failures",
    "cut",
    "exit",
    "deviations",
];

/// How a crossing is answered. `works` names its proof, `refused` names what
/// refuses it, `n/a` says why it does not apply.
const ANSWERS: &[(&str, &str)] = &[("works", "proof"), ("refused", "by"), ("n/a", "why")];

/// Keys whose values are identifiers, paths or proof markers rather than
/// sentences, and so are not held to the word ceiling.
const NOT_PROSE: &[&str] = &[
    "id",
    "kind",
    "proof",
    "ref",
    "sources",
    "mechanism",
    "answer",
    "touches",
    "untouched",
    "who",
    "when",
    "base",
    "symbol",
];

/// A declaration split off the top of a plan.
pub struct Frontmatter {
    /// The YAML between the fences.
    pub yaml: String,
    /// 1-based line of the first YAML line in the file.
    pub first_line: usize,
    /// Everything after the closing fence.
    pub body: String,
}

/// The YAML block a plan opens with, if it opens with one.
pub fn frontmatter(text: &str) -> Option<Frontmatter> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut yaml = String::new();
    for line in lines.by_ref() {
        if line.trim_end() == "---" {
            return Some(Frontmatter {
                yaml,
                first_line: 2,
                body: lines.collect(),
            });
        }
        yaml.push_str(line);
    }
    None
}

/// The parsed declaration, or why it does not parse.
pub fn declaration(text: &str) -> Option<Result<(Frontmatter, Mapping), String>> {
    let front = frontmatter(text)?;
    Some(match serde_yaml::from_str::<Value>(&front.yaml) {
        Ok(Value::Mapping(map)) => Ok((front, map)),
        Ok(Value::Null) => Ok((front, Mapping::new())),
        Ok(_) => Err("the frontmatter is not a mapping".to_string()),
        Err(e) => Err(format!("the frontmatter is not valid YAML: {e}")),
    })
}

/// A criterion a declaration carries, as the older plan rules read one.
pub struct DeclaredCriterion {
    pub line: usize,
    pub text: String,
    pub proof: Option<String>,
}

/// Every `criteria` entry of a declaration, with the line its `id` sits on.
pub fn declared_criteria(text: &str) -> Vec<DeclaredCriterion> {
    let Some(Ok((front, map))) = declaration(text) else {
        return Vec::new();
    };
    let Some(Value::Sequence(items)) = map.get("criteria") else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| {
            let id = string_at(item, "id").unwrap_or_default();
            DeclaredCriterion {
                line: line_of(&front, &id),
                text: string_at(item, "statement").unwrap_or_else(|| id.clone()),
                proof: string_at(item, "proof"),
            }
        })
        .collect()
}

/// Whether a declaration states its exit condition.
pub fn declares_exit(text: &str) -> bool {
    matches!(
        declaration(text),
        Some(Ok((_, map))) if string_value(map.get("exit")).is_some_and(|s| !s.trim().is_empty())
    )
}

fn string_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn string_at(item: &Value, key: &str) -> Option<String> {
    string_value(item.get(key))
}

/// The file line an id is declared on, so a finding points at the entry
/// rather than at the top of the plan.
fn line_of(front: &Frontmatter, id: &str) -> usize {
    if id.is_empty() {
        return front.first_line;
    }
    front
        .yaml
        .lines()
        .position(|line| {
            let line = line.trim_start().trim_start_matches(['-', '{', ' ']);
            line.strip_prefix("id:")
                .map(|rest| {
                    rest.trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .split([',', '}'])
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        == id
                })
                .unwrap_or(false)
        })
        .map(|index| front.first_line + index)
        .unwrap_or(front.first_line)
}

/// One mechanism a repository names once, with the code it lives in.
struct Mechanism {
    id: String,
    paths: Vec<String>,
}

fn mechanisms(ctx: &Ctx, path: &str) -> Result<Vec<Mechanism>, String> {
    let text = std::fs::read_to_string(ctx.root.join(path))
        .map_err(|e| format!("the mechanisms catalog `{path}` cannot be read: {e}"))?;
    parse_mechanisms(&text, path)
}

/// Every entry is held to its shape: a catalog that drops a malformed entry
/// silently drops the crossing obligation it was there to create.
fn parse_mechanisms(text: &str, path: &str) -> Result<Vec<Mechanism>, String> {
    let value: Value = serde_yaml::from_str(text)
        .map_err(|e| format!("the mechanisms catalog `{path}` is not valid YAML: {e}"))?;
    let Some(Value::Sequence(items)) = value.get("mechanisms") else {
        return Err(format!(
            "the mechanisms catalog `{path}` has no `mechanisms:` list"
        ));
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let at = format!("the mechanisms catalog `{path}`, entry {}", index + 1);
        let id = string_at(item, "id").ok_or_else(|| format!("{at} has no `id`"))?;
        if !seen.insert(id.clone()) {
            return Err(format!("{at} repeats the id `{id}`"));
        }
        let Some(Value::Sequence(globs)) = item.get("paths") else {
            return Err(format!("{at} (`{id}`) has no `paths:` list"));
        };
        let mut paths = Vec::new();
        for glob in globs {
            let Value::String(glob) = glob else {
                return Err(format!("{at} (`{id}`) has a path that is not a string"));
            };
            scan::globs(std::slice::from_ref(glob))
                .map_err(|e| format!("{at} (`{id}`) has an invalid glob `{glob}`: {e}"))?;
            paths.push(glob.clone());
        }
        if paths.is_empty() {
            return Err(format!("{at} (`{id}`) lists no paths"));
        }
        out.push(Mechanism { id, paths });
    }
    Ok(out)
}

fn strings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Sequence(items)) => {
            items.iter().filter_map(|v| string_value(Some(v))).collect()
        }
        Some(other) => string_value(Some(other)).into_iter().collect(),
        None => Vec::new(),
    }
}

/// What one plan says wrong, collected before it becomes findings.
struct Problems {
    rel: String,
    found: Vec<(String, usize, String, String)>,
}

impl Problems {
    fn push(
        &mut self,
        key: &str,
        line: usize,
        message: impl Into<String>,
        expected: impl Into<String>,
    ) {
        self.found
            .push((key.to_string(), line, message.into(), expected.into()));
    }
}

/// `L4.PLAN_IS_A_LINKED_DECLARATION`.
pub fn plan_declaration(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let plans = scan::select(ctx.files, &opts.scope, &opts.exclude)?;
    let mut findings = Vec::new();
    if plans.is_empty() {
        return Ok(findings);
    }
    let catalog = opts.mechanisms.as_ref().map(|path| mechanisms(ctx, path));
    if let Some(Err(why)) = &catalog {
        findings.push(
            Finding::new(
                &rule.id,
                rule.severity,
                opts.mechanisms.clone().unwrap_or_default(),
                "mechanisms-catalog".to_string(),
                why.clone(),
            )
            .expected("a YAML file with a `mechanisms:` list of `{ id, paths }`"),
        );
    }
    let catalog = catalog.and_then(Result::ok);
    for file in plans {
        let Ok(text) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        let mut problems = Problems {
            rel: file.rel.clone(),
            found: Vec::new(),
        };
        judge(&text, opts, ctx.files, catalog.as_deref(), &mut problems);
        for (key, line, message, expected) in problems.found {
            findings.push(
                Finding::new(
                    &rule.id,
                    rule.severity,
                    format!("{}:{line}", problems.rel),
                    format!("{key}:{}", problems.rel),
                    message,
                )
                .expected(expected),
            );
        }
    }
    Ok(findings)
}

fn judge(
    text: &str,
    opts: &Options,
    files: &[scan::SourceFile],
    catalog: Option<&[Mechanism]>,
    out: &mut Problems,
) {
    let (front, map) = match declaration(text) {
        None => {
            if opts.require_declaration {
                out.push(
                    "undeclared",
                    1,
                    "this plan is prose with no declaration",
                    "a YAML frontmatter block between `---` fences carrying its criteria and exit",
                );
            }
            return;
        }
        Some(Err(why)) => {
            out.push(
                "unparsable",
                1,
                why,
                "a YAML mapping between the `---` fences",
            );
            return;
        }
        Some(Ok(parsed)) => parsed,
    };
    closed_keys(&front, &map, out);
    links(&front, &map, out);
    crossings(&front, &map, files, catalog, out);
    if let Some(max) = opts.max_words {
        word_ceiling(&front, &Value::Mapping(map.clone()), None, max, out);
    }
    if let Some(max) = opts.max_body_lines {
        body_ceiling(&front, max, out);
    }
}

fn closed_keys(front: &Frontmatter, map: &Mapping, out: &mut Problems) {
    for key in map.keys() {
        let name = string_value(Some(key)).unwrap_or_default();
        if !DECLARATION_KEYS.contains(&name.as_str()) {
            out.push(
                &format!("unknown-key:{name}"),
                front.first_line,
                format!("`{name}` is not a key a declaration carries"),
                format!("one of: {}", DECLARATION_KEYS.join(", ")),
            );
        }
    }
}

/// Every criterion names a source that exists, and every source is named by
/// a criterion. An orphan on either side is a line that changes nothing.
fn links(front: &Frontmatter, map: &Mapping, out: &mut Problems) {
    let items = |key: &str| match map.get(key) {
        Some(Value::Sequence(items)) => items.clone(),
        _ => Vec::new(),
    };
    let mut seen = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for source in items("sources") {
        let Some(id) = string_at(&source, "id") else {
            out.push(
                "source-id",
                front.first_line,
                "a source carries no `id`",
                "`id: S1`",
            );
            continue;
        };
        if string_at(&source, "kind").is_none() {
            out.push(
                &format!("source-kind:{id}"),
                line_of(front, &id),
                format!("source `{id}` names no `kind`"),
                "`kind:` decision, reference, ticket, friction or what it is",
            );
        }
        if !seen.insert(id.clone()) {
            out.push(
                &format!("duplicate:{id}"),
                line_of(front, &id),
                format!("`{id}` is declared twice"),
                "one entry per id",
            );
        }
        sources.insert(id);
    }
    let mut cited = BTreeSet::new();
    for criterion in items("criteria") {
        let Some(id) = string_at(&criterion, "id") else {
            out.push(
                "criterion-id",
                front.first_line,
                "a criterion carries no `id`",
                "`id: A1`",
            );
            continue;
        };
        let line = line_of(front, &id);
        if !seen.insert(id.clone()) {
            out.push(
                &format!("duplicate:{id}"),
                line,
                format!("`{id}` is declared twice"),
                "one entry per id",
            );
        }
        if string_at(&criterion, "statement").is_none_or(|s| s.trim().is_empty()) {
            out.push(
                &format!("statement:{id}"),
                line,
                format!("criterion `{id}` states nothing"),
                "`statement:` the behaviour that has to hold",
            );
        }
        let named = strings(criterion.get("sources"));
        if named.is_empty() {
            out.push(
                &format!("sourceless:{id}"),
                line,
                format!("criterion `{id}` names no source: nobody asked for it"),
                "`sources: [S1]`, or cut the criterion",
            );
        }
        for source in named {
            if !sources.contains(&source) {
                out.push(
                    &format!("dangling:{id}:{source}"),
                    line,
                    format!(
                        "criterion `{id}` names source `{source}`, which the plan does not declare"
                    ),
                    format!("a `sources` entry with `id: {source}`"),
                );
            }
            cited.insert(source);
        }
    }
    for source in sources.difference(&cited) {
        out.push(
            &format!("orphan:{source}"),
            line_of(front, source),
            format!("source `{source}` is named by no criterion: it changes nothing that is built"),
            "a criterion that cites it, or remove it",
        );
    }
}

/// Every crossing is answered, and every mechanism the plan's files reach is
/// crossed. The list a plan must answer is derived, not remembered.
fn crossings(
    front: &Frontmatter,
    map: &Mapping,
    files: &[scan::SourceFile],
    catalog: Option<&[Mechanism]>,
    out: &mut Problems,
) {
    let crosses = match map.get("crosses") {
        Some(Value::Sequence(items)) => items.clone(),
        _ => Vec::new(),
    };
    let mut answered = BTreeSet::new();
    for crossing in &crosses {
        if let Some(id) = answered_crossing(front, crossing, catalog, out) {
            answered.insert(id);
        }
    }
    if let Some(known) = catalog {
        uncrossed(
            front,
            &touched_paths(files, &strings(map.get("touches"))),
            known,
            &answered,
            out,
        );
    }
}

fn answered_crossing(
    front: &Frontmatter,
    crossing: &Value,
    catalog: Option<&[Mechanism]>,
    out: &mut Problems,
) -> Option<String> {
    let Some(id) = string_at(crossing, "mechanism") else {
        out.push(
            "crossing-mechanism",
            front.first_line,
            "a crossing names no `mechanism`",
            "`mechanism:` an id from the mechanisms catalog",
        );
        return None;
    };
    if catalog.is_some_and(|known| !known.iter().any(|m| m.id == id)) {
        out.push(
            &format!("unknown-mechanism:{id}"),
            front.first_line,
            format!("`{id}` is not a mechanism the catalog names"),
            "an id from the mechanisms catalog, or a new entry there",
        );
    }
    let answer = string_at(crossing, "answer").unwrap_or_default();
    match ANSWERS.iter().find(|(name, _)| *name == answer) {
        None => out.push(
            &format!("crossing-answer:{id}"),
            front.first_line,
            format!("crossing `{id}` answers `{answer}`"),
            "`answer:` works, refused or n/a",
        ),
        Some((name, needs)) if string_at(crossing, needs).is_none_or(|s| s.trim().is_empty()) => {
            out.push(
                &format!("crossing-unbacked:{id}"),
                front.first_line,
                format!("crossing `{id}` answers `{name}` with no `{needs}`"),
                format!("`{needs}:` on the crossing"),
            )
        }
        Some(_) => {}
    }
    Some(id)
}

fn uncrossed(
    front: &Frontmatter,
    touched: &Touched,
    known: &[Mechanism],
    answered: &BTreeSet<String>,
    out: &mut Problems,
) {
    for mechanism in known.iter().filter(|m| !answered.contains(&m.id)) {
        if let Some(path) = touched.reaches(mechanism) {
            out.push(
                &format!("uncrossed:{}", mechanism.id),
                front.first_line,
                format!(
                    "the plan touches `{path}`, where mechanism `{}` lives, and never says what happens to it",
                    mechanism.id
                ),
                format!("a `crosses` entry for `{}`: works, refused or n/a", mechanism.id),
            );
        }
    }
}

/// What a plan's `touches` reach: the repository files its globs match, any
/// literal path it names that does not exist yet (a plan that adds a file
/// still touches it), and the globs that match nothing yet.
struct Touched {
    paths: Vec<String>,
    unmatched: Vec<String>,
}

impl Touched {
    /// The touched path, or the not-yet-matching glob, that lands where the
    /// mechanism lives. A glob that matches no file yet reaches a mechanism
    /// when either glob matches a concrete path drawn from the other, so
    /// `src/ledger/**` reaches a mechanism at `src/ledger/**` before the
    /// directory exists, and `packages/server/**` does not reach
    /// `packages/*/workflow.mmd`.
    fn reaches(&self, mechanism: &Mechanism) -> Option<String> {
        let lives = scan::globs(&mechanism.paths).ok()?;
        if let Some(path) = self.paths.iter().find(|p| lives.is_match(p)) {
            return Some(path.clone());
        }
        self.unmatched
            .iter()
            .find(|glob| {
                let touched = scan::globs(std::slice::from_ref(*glob));
                lives.is_match(concrete(glob))
                    || touched
                        .is_ok_and(|t| mechanism.paths.iter().any(|m| t.is_match(concrete(m))))
            })
            .cloned()
    }
}

/// One path a glob matches: a brace set becomes its first alternative, a
/// character class and every run of wildcards a plain name.
fn concrete(glob: &str) -> String {
    let braces = Regex::new(r"\{([^,}]*)[^}]*\}").expect("a fixed pattern compiles");
    let classes = Regex::new(r"\[[^\]]*\]").expect("a fixed pattern compiles");
    let wildcards = Regex::new(r"[*?]+").expect("a fixed pattern compiles");
    let glob = braces.replace_all(glob, "$1");
    let glob = classes.replace_all(&glob, "x");
    wildcards.replace_all(&glob, "x").into_owned()
}

fn touched_paths(files: &[scan::SourceFile], touches: &[String]) -> Touched {
    let mut paths = BTreeSet::new();
    let mut unmatched = Vec::new();
    for pattern in touches {
        if !pattern.contains(['*', '?', '[', '{']) {
            paths.insert(pattern.clone());
            continue;
        }
        let matched: Vec<_> = scan::globs(std::slice::from_ref(pattern))
            .map(|set| files.iter().filter(|f| set.is_match(&f.rel)).collect())
            .unwrap_or_default();
        if matched.is_empty() {
            unmatched.push(pattern.clone());
        }
        paths.extend(matched.into_iter().map(|f| f.rel.clone()));
    }
    Touched {
        paths: paths.into_iter().collect(),
        unmatched,
    }
}

fn word_ceiling(
    front: &Frontmatter,
    value: &Value,
    key: Option<&str>,
    max: usize,
    out: &mut Problems,
) {
    if key.is_some_and(|k| NOT_PROSE.contains(&k)) && holds_no_prose(value) {
        return;
    }
    match value {
        Value::String(s) => {
            let words = s.split_whitespace().count();
            if words > max {
                let head: String = s.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
                out.push(
                    &format!("wordy:{}", head.replace(' ', "_")),
                    front.first_line,
                    format!("\"{head} …\" runs to {words} words, past the {max}-word ceiling"),
                    format!("at most {max} words; split it, or move the detail into the code"),
                );
            }
        }
        Value::Sequence(items) => items
            .iter()
            .for_each(|item| word_ceiling(front, item, key, max, out)),
        Value::Mapping(map) => {
            for (k, v) in map {
                let name = string_value(Some(k));
                word_ceiling(front, v, name.as_deref(), max, out);
            }
        }
        _ => {}
    }
}

/// A scalar, or a list of scalars: an identifier, a path, an id list. A list
/// of entries (top-level `sources`) is traversed, so its `said` is held too.
fn holds_no_prose(value: &Value) -> bool {
    match value {
        Value::Sequence(items) => items
            .iter()
            .all(|item| !matches!(item, Value::Mapping(_) | Value::Sequence(_))),
        Value::Mapping(_) => false,
        _ => true,
    }
}

/// Prose under the declaration is a few lines of why, not a second plan.
fn body_ceiling(front: &Frontmatter, max: usize, out: &mut Problems) {
    let prose = front
        .body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count();
    if prose > max {
        let line = front.first_line + front.yaml.lines().count() + 1;
        out.push(
            "body",
            line,
            format!("{prose} lines of prose sit under the declaration, past the ceiling of {max}"),
            format!(
                "at most {max} lines of why; the rest belongs in the declaration or in the code"
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plan_with_no_fences_has_no_declaration() {
        assert!(frontmatter("# Plan\n\nExit condition: done.\n").is_none());
        assert!(frontmatter("---\nexit: never closed\n").is_none());
    }

    #[test]
    fn criteria_are_read_from_the_declaration_with_their_line() {
        let text = "---\nsources:\n  - { id: S1, kind: decision }\ncriteria:\n  - { id: A1, statement: \"a guest pays\", proof: \"test:tests/pay.rs\", sources: [S1] }\n  - id: A2\n    statement: refunds reconcile\n    sources: [S1]\nexit: checkout serves traffic\n---\n# Checkout\n";
        let criteria = declared_criteria(text);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0].line, 5);
        assert_eq!(criteria[0].proof.as_deref(), Some("test:tests/pay.rs"));
        assert_eq!(criteria[1].line, 6);
        assert_eq!(criteria[1].proof, None);
        assert!(declares_exit(text));
    }

    /// The mutation fixture, read refusal by refusal. `sf verify` only asks
    /// that the rule fires somewhere in it; this asks that each plan fails
    /// for exactly its own reason and that the linked control stays quiet.
    #[test]
    fn the_fixture_trips_each_refusal_and_spares_the_linked_plan() {
        let fixture = crate::fixtures::FIXTURES
            .iter()
            .find(|f| f.rule == "L4.PLAN_IS_A_LINKED_DECLARATION")
            .expect("the rule has a fixture");
        let dedented: String = fixture
            .policy_extra
            .lines()
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        let opts: Options = serde_yaml::from_str(&dedented).expect("the fixture's options parse");
        let files: Vec<scan::SourceFile> = fixture
            .files
            .iter()
            .map(|(rel, _)| scan::SourceFile {
                rel: rel.to_string(),
                abs: std::path::PathBuf::from(rel),
            })
            .collect();
        let (_, catalog) = fixture
            .files
            .iter()
            .find(|(rel, _)| *rel == "docs/mechanisms.yaml")
            .expect("the fixture carries a catalog");
        let catalog =
            parse_mechanisms(catalog, "docs/mechanisms.yaml").expect("the catalog is valid");
        let keys = |plan: &str| {
            let (_, text) = fixture
                .files
                .iter()
                .find(|(rel, _)| *rel == plan)
                .expect("the plan is in the fixture");
            let mut problems = Problems {
                rel: plan.to_string(),
                found: Vec::new(),
            };
            judge(text, &opts, &files, Some(&catalog), &mut problems);
            let mut keys: Vec<String> = problems.found.into_iter().map(|(key, ..)| key).collect();
            keys.sort();
            keys
        };
        assert_eq!(keys("plans/prose.md"), vec!["undeclared"]);
        assert_eq!(
            keys("plans/unlinked.md"),
            vec!["orphan:S2", "sourceless:A1", "unknown-key:notes"]
        );
        assert_eq!(
            keys("plans/uncrossed.md"),
            vec!["crossing-unbacked:ledger", "uncrossed:sessions"]
        );
        assert_eq!(keys("plans/wordy.md").len(), 2);
        assert!(
            keys("plans/wordy.md")
                .iter()
                .any(|k| k.starts_with("wordy:"))
        );
        assert!(keys("plans/wordy.md").contains(&"body".to_string()));
        assert_eq!(keys("plans/linked.md"), Vec::<String>::new());
    }

    #[test]
    fn a_malformed_catalog_entry_is_refused_rather_than_dropped() {
        for (catalog, says) in [
            ("mechanisms:\n  - paths: [\"src/**\"]\n", "has no `id`"),
            ("mechanisms:\n  - id: ledger\n", "has no `paths:` list"),
            (
                "mechanisms:\n  - id: ledger\n    paths: []\n",
                "lists no paths",
            ),
            (
                "mechanisms:\n  - id: ledger\n    paths: [\"src/[\"]\n",
                "invalid glob",
            ),
            (
                "mechanisms:\n  - { id: a, paths: [x] }\n  - { id: a, paths: [y] }\n",
                "repeats the id",
            ),
        ] {
            let error = parse_mechanisms(catalog, "m.yaml")
                .err()
                .unwrap_or_default();
            assert!(
                error.contains(says),
                "{catalog:?} should say {says:?}, said {error:?}"
            );
        }
    }

    #[test]
    fn a_glob_that_matches_nothing_yet_still_reaches_its_mechanism() {
        let touched = touched_paths(&[], &["src/ledger/**".to_string()]);
        let ledger = Mechanism {
            id: "ledger".into(),
            paths: vec!["src/ledger/**".into()],
        };
        let other = Mechanism {
            id: "sessions".into(),
            paths: vec!["src/sessions/**".into()],
        };
        assert_eq!(touched.reaches(&ledger), Some("src/ledger/**".to_string()));
        assert_eq!(concrete("src/{a,b}/[ab]*/**/x.ts"), "src/a/xx/x/x.ts");
        assert_eq!(touched.reaches(&other), None);
        let diagram = Mechanism {
            id: "diagram".into(),
            paths: vec!["packages/*/workflow.mmd".into()],
        };
        let server = touched_paths(&[], &["packages/server/**".to_string()]);
        assert_eq!(
            server.reaches(&diagram),
            None,
            "a sibling directory under a wildcard is not reached"
        );
        let broad = touched_paths(&[], &["src/**".to_string()]);
        assert_eq!(
            broad.reaches(&ledger),
            Some("src/**".to_string()),
            "a wider glob reaches what it covers"
        );
    }

    #[test]
    fn a_source_sentence_is_held_to_the_word_ceiling() {
        let text = "---\nsources:\n  - { id: S1, kind: decision, said: \"one two three four five six\" }\ncriteria:\n  - { id: A1, statement: ok, sources: [S1] }\nexit: done\n---\n";
        let opts = Options {
            max_words: Some(5),
            ..Options::default()
        };
        let mut problems = Problems {
            rel: "p.md".into(),
            found: Vec::new(),
        };
        judge(text, &opts, &[], None, &mut problems);
        assert!(
            problems
                .found
                .iter()
                .any(|(key, ..)| key.starts_with("wordy:one_two"))
        );
    }

    #[test]
    fn an_empty_exit_is_no_exit() {
        assert!(!declares_exit("---\nexit: \"\"\n---\n"));
    }
}
