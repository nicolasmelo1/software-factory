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
use serde_yaml::{Mapping, Value};
use std::collections::BTreeSet;

/// The keys a declaration may carry. Closed on purpose: a free key is where
/// the context nobody owns grows back.
pub const DECLARATION_KEYS: &[&str] = &[
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
    let value: Value = serde_yaml::from_str(&text)
        .map_err(|e| format!("the mechanisms catalog `{path}` is not valid YAML: {e}"))?;
    let Some(Value::Sequence(items)) = value.get("mechanisms") else {
        return Err(format!(
            "the mechanisms catalog `{path}` has no `mechanisms:` list"
        ));
    };
    Ok(items
        .iter()
        .filter_map(|item| {
            Some(Mechanism {
                id: string_at(item, "id")?,
                paths: strings(item.get("paths")),
            })
        })
        .collect())
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
    let catalog = opts.mechanisms.as_ref().map(|path| mechanisms(ctx, path));
    let mut findings = Vec::new();
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
    for file in scan::select(ctx.files, &opts.scope, &opts.exclude)? {
        let Ok(text) = std::fs::read_to_string(&file.abs) else {
            continue;
        };
        let mut problems = Problems {
            rel: file.rel.clone(),
            found: Vec::new(),
        };
        judge(&text, opts, ctx, catalog.as_deref(), &mut problems);
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

fn judge(text: &str, opts: &Options, ctx: &Ctx, catalog: Option<&[Mechanism]>, out: &mut Problems) {
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
    crossings(&front, &map, ctx, catalog, out);
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
    ctx: &Ctx,
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
            &touched_paths(ctx, &strings(map.get("touches"))),
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
    touched: &[String],
    known: &[Mechanism],
    answered: &BTreeSet<String>,
    out: &mut Problems,
) {
    for mechanism in known
        .iter()
        .filter(|m| !answered.contains(&m.id) && !m.paths.is_empty())
    {
        let Ok(set) = scan::globs(&mechanism.paths) else {
            continue;
        };
        if let Some(path) = touched.iter().find(|p| set.is_match(p)) {
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

/// The repository files a plan's `touches` reach, plus any literal path it
/// names that does not exist yet: a plan that adds a file still touches it.
fn touched_paths(ctx: &Ctx, touches: &[String]) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for pattern in touches {
        if let Ok(set) = scan::globs(std::slice::from_ref(pattern)) {
            for file in ctx.files.iter().filter(|f| set.is_match(&f.rel)) {
                paths.insert(file.rel.clone());
            }
        }
        if !pattern.contains(['*', '?', '[', '{']) {
            paths.insert(pattern.clone());
        }
    }
    paths.into_iter().collect()
}

fn word_ceiling(
    front: &Frontmatter,
    value: &Value,
    key: Option<&str>,
    max: usize,
    out: &mut Problems,
) {
    if key.is_some_and(|k| NOT_PROSE.contains(&k)) {
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

    #[test]
    fn an_empty_exit_is_no_exit() {
        assert!(!declares_exit("---\nexit: \"\"\n---\n"));
    }
}
