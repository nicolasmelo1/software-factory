//! A plan written as a declaration: YAML frontmatter carrying what it
//! promises, where each promise comes from, and what it crosses and refuses.
//!
//! The older plan rules read criteria and the exit through [`declared_criteria`]
//! and [`declares_exit`], so a repository can move one plan at a time.
//! [`plan_declaration`] is `L4.PLAN_IS_A_LINKED_DECLARATION`.
//!
//! A plan is also held to the work it is in flight for: when the run knows
//! what changed, every changed file has to be declared, incidental or a
//! recorded deviation, and the mechanisms those files reach join the ones its
//! `touches` reach. What a plan declared is what its author remembered; what
//! the diff holds is what happened.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::policy::{Options, POLICY_PATH};
use crate::scan;
use anyhow::Result;
use globset::GlobSet;
use regex::Regex;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

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
    let scoped = scan::select(ctx.files, &opts.scope, &opts.exclude)?;
    let delivered = if opts.delivered.is_empty() {
        Vec::new()
    } else {
        scan::select(ctx.files, &opts.delivered, &opts.exclude)?
    };
    let mut findings = Vec::new();
    if scoped.is_empty() && delivered.is_empty() {
        return Ok(findings);
    }
    let changed = changed_or_reported(rule, ctx, &mut findings);
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
    let mut plans: Vec<(String, String)> = scoped
        .iter()
        .filter_map(|file| Some((file.rel.clone(), std::fs::read_to_string(&file.abs).ok()?)))
        .collect();
    let delivering = delivered_in_flight(&delivered, changed.as_deref(), &plans);
    plans.extend(delivering);
    for problems in judge_plans(
        &plans,
        changed.as_deref(),
        opts,
        ctx.files,
        catalog.as_deref(),
    )? {
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

/// The work's changed set, or `None` with the reason pushed as a finding
/// when git could not say: an unreadable change is not an empty one.
fn changed_or_reported(rule: &Rule, ctx: &Ctx, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    work_changed(
        ctx.root,
        ctx.changed.as_deref(),
        ctx.policy.project.default_branch.as_deref(),
    )
    .unwrap_or_else(|why| {
        findings.push(
            Finding::new(&rule.id, rule.severity, POLICY_PATH, "changed-set", why).expected(
                "a changed set git can read: `--changed <ref>`, or a `project.default_branch` this repository has",
            ),
        );
        None
    })
}

/// A delivered plan is judged only while the change that delivers or edits
/// it is under review, and only if it still opens with a declaration, one
/// that parses or not.
fn delivered_in_flight(
    delivered: &[&scan::SourceFile],
    changed: Option<&[String]>,
    scoped: &[(String, String)],
) -> Vec<(String, String)> {
    let Some(changed) = changed else {
        return Vec::new();
    };
    delivered
        .iter()
        .filter(|file| changed.binary_search(&file.rel).is_ok())
        .filter(|file| !scoped.iter().any(|(rel, _)| *rel == file.rel))
        .filter_map(|file| {
            let text = std::fs::read_to_string(&file.abs).ok()?;
            // Unparsable is still a declaration: it is judged, and refused,
            // rather than dropped with the work it carries.
            declaration(&text)
                .is_some()
                .then(|| (file.rel.clone(), text))
        })
        .collect()
}

/// Every plan judged in path order, each in-flight one also against the
/// work's changed set.
fn judge_plans(
    plans: &[(String, String)],
    changed: Option<&[String]>,
    opts: &Options,
    files: &[scan::SourceFile],
    catalog: Option<&[Mechanism]>,
) -> Result<Vec<Problems>> {
    let work = match changed {
        Some(changed) => Some(Work::new(plans, changed, opts)?),
        None => None,
    };
    let uncovered = work
        .as_ref()
        .map(|work| work.uncovered(touches_cover))
        .unwrap_or_default();
    let mut ordered: Vec<&(String, String)> = plans.iter().collect();
    ordered.sort();
    let mut out = Vec::new();
    for (rel, text) in ordered {
        let owed = work
            .as_ref()
            .filter(|work| work.flights.iter().any(|f| f.rel == *rel))
            .map(|work| work.owed(rel, &uncovered, touches_cover));
        let mut problems = Problems {
            rel: rel.clone(),
            found: Vec::new(),
        };
        judge(text, opts, files, catalog, owed.as_ref(), &mut problems);
        out.push(problems);
    }
    Ok(out)
}

/// The paths the work under review changed, sorted and without repeats.
/// `--changed` when the run was given it; otherwise, under
/// `project.default_branch`, the tree against its merge-base with that
/// branch, which needs nothing but local refs and so reads the same in a
/// pre-commit hook as it does later; otherwise unknown, and nothing is added.
/// A git failure is an error, never an empty set.
fn work_changed(
    root: &Path,
    changed: Option<&[String]>,
    default_branch: Option<&str>,
) -> Result<Option<Vec<String>>, String> {
    if let Some(changed) = changed {
        return Ok(Some(sorted(changed.iter().cloned())));
    }
    let Some(branch) = default_branch else {
        return Ok(None);
    };
    let base = git(root, &["merge-base", "HEAD", branch])?;
    Ok(Some(sorted(changed_since(root, base.trim())?.into_iter())))
}

/// Every path the tree changed since `base`, committed or not, as git lists
/// them: NUL-delimited, so a path git would quote on a line arrives as the
/// path itself. `--changed` reads through here too, so both producers of a
/// changed set agree on what a path is. A git failure is an error.
pub fn changed_since(root: &Path, base: &str) -> Result<Vec<String>, String> {
    let listed = git(root, &["diff", "--name-only", "-z", base, "--"])?;
    Ok(listed
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect())
}

/// Run git against the repository at `root` and nothing else: a hook exports
/// `GIT_DIR` and `GIT_INDEX_FILE`, and a run must not read whichever
/// repository its environment happens to point at.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let shown = format!("git {}", args.join(" "));
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| format!("`{shown}` could not run: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`{shown}` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| format!("`{shown}` printed a path that is not UTF-8"))
}

fn sorted(paths: impl Iterator<Item = String>) -> Vec<String> {
    paths.collect::<BTreeSet<_>>().into_iter().collect()
}

/// One plan in flight: its own file is in the changed set.
struct Flight {
    rel: String,
    touches: Vec<String>,
    deviations: Vec<String>,
}

/// The work under review, as the in-flight plans have to account for it.
struct Work {
    /// Sorted and without repeats.
    changed: Vec<String>,
    flights: Vec<Flight>,
    /// The rule's `incidental` globs.
    incidental: GlobSet,
    /// The rule's plan scope: a plan file the change edited is in flight, and
    /// one it removed was delivered or dropped, so neither is work to declare.
    plans: GlobSet,
    plans_excluded: GlobSet,
}

/// Whether a plan's `touches` declare a path. Passed in rather than called by
/// name so the property below can plant a defective matcher.
type Covers = fn(&[String], &str) -> bool;

/// What one in-flight plan owes the diff.
#[derive(Default)]
struct Owed {
    /// Changed files no in-flight plan accounts for.
    untouched: Vec<String>,
    /// Changed, non-incidental files whose mechanisms this plan answers:
    /// those it declares or names as a deviation, and those nobody does.
    reached: Vec<String>,
}

impl Work {
    fn new(plans: &[(String, String)], changed: &[String], opts: &Options) -> Result<Self> {
        let changed = sorted(changed.iter().cloned());
        let flights = plans
            .iter()
            .filter(|(rel, _)| changed.binary_search(rel).is_ok())
            .map(|(rel, text)| {
                let map = match declaration(text) {
                    Some(Ok((_, map))) => map,
                    _ => Mapping::new(),
                };
                Flight {
                    rel: rel.clone(),
                    touches: strings(map.get("touches")),
                    deviations: strings(map.get("deviations")),
                }
            })
            .collect();
        Ok(Work {
            changed,
            flights,
            incidental: scan::globs(&opts.incidental)?,
            plans: scan::globs(&opts.scope)?,
            plans_excluded: scan::globs(&opts.exclude)?,
        })
    }

    fn is_incidental(&self, path: &str) -> bool {
        self.incidental.is_match(path)
            || (self.plans.is_match(path) && !self.plans_excluded.is_match(path))
            || self.flights.iter().any(|flight| flight.rel == path)
    }

    /// Changed, non-incidental files paired with whether any in-flight plan
    /// declares them or names them under `deviations`.
    fn accounted(&self, covers: Covers) -> impl Iterator<Item = (&String, bool)> {
        self.changed
            .iter()
            .filter(|path| !self.is_incidental(path))
            .map(move |path| {
                let accounted = self.flights.iter().any(|flight| {
                    covers(&flight.touches, path) || mentions(&flight.deviations, path)
                });
                (path, accounted)
            })
    }

    /// Every changed file no in-flight plan declares, lists as incidental or
    /// names as a deviation.
    fn uncovered(&self, covers: Covers) -> Vec<String> {
        self.accounted(covers)
            .filter(|(_, accounted)| !accounted)
            .map(|(path, _)| path.clone())
            .collect()
    }

    fn owed(&self, rel: &str, uncovered: &[String], covers: Covers) -> Owed {
        let Some(flight) = self.flights.iter().find(|flight| flight.rel == rel) else {
            return Owed::default();
        };
        let reached = self
            .accounted(covers)
            .filter(|(path, accounted)| {
                !accounted || covers(&flight.touches, path) || mentions(&flight.deviations, path)
            })
            .map(|(path, _)| path.clone())
            .collect();
        Owed {
            untouched: uncovered.to_vec(),
            reached,
        }
    }
}

/// The matcher `touches` is read with everywhere in this file: a glob
/// matches through the glob set, a literal names exactly one path.
fn touches_cover(touches: &[String], path: &str) -> bool {
    touches.iter().any(|touch| {
        if is_glob(touch) {
            scan::globs(std::slice::from_ref(touch)).is_ok_and(|set| set.is_match(path))
        } else {
            touch == path
        }
    })
}

fn is_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?', '[', '{'])
}

/// Whether a deviation names a path literally, as a whole path: `src/a.rs`
/// is not mentioned by `src/a.rs.bak`, `src/a.rs~` or `lib/src/a.rs`. A
/// sentence may end on it, so a trailing full stop is not part of the path.
fn mentions(deviations: &[String], path: &str) -> bool {
    let part =
        |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '~' | '+' | '@');
    deviations.iter().any(|deviation| {
        deviation.match_indices(path).any(|(at, _)| {
            let before = deviation[..at].chars().next_back();
            let mut after = deviation[at + path.len()..].chars();
            let ends = match after.next() {
                None => true,
                Some('.') => after.next().is_none_or(|c| !part(c)),
                Some(c) => !part(c),
            };
            !before.is_some_and(part) && ends
        })
    })
}

fn judge(
    text: &str,
    opts: &Options,
    files: &[scan::SourceFile],
    catalog: Option<&[Mechanism]>,
    owed: Option<&Owed>,
    out: &mut Problems,
) {
    let parsed = declaration(text);
    if let Some(owed) = owed {
        untouched(&owed.untouched, &parsed, out);
    }
    let (front, map) = match parsed {
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
    let reached = owed.map(|owed| owed.reached.as_slice()).unwrap_or_default();
    crossings(&front, &map, files, reached, catalog, out);
    if let Some(max) = opts.max_words {
        word_ceiling(&front, &Value::Mapping(map.clone()), None, max, out);
    }
    if let Some(max) = opts.max_body_lines {
        body_ceiling(&front, max, out);
    }
}

/// One finding per changed file nobody accounts for, in path order, on the
/// plan's `touches` line when it has one.
fn untouched(
    paths: &[String],
    parsed: &Option<Result<(Frontmatter, Mapping), String>>,
    out: &mut Problems,
) {
    let line = match parsed {
        Some(Ok((front, _))) => key_line(front, "touches"),
        _ => 1,
    };
    for path in paths {
        out.push(
            &format!("untouched:{path}"),
            line,
            format!("the work changed `{path}`, which no plan in flight declares"),
            "add it to `touches`, list it under the rule's `incidental`, or name it under `deviations`",
        );
    }
}

/// The file line a top-level key opens on, or the declaration's first line.
fn key_line(front: &Frontmatter, key: &str) -> usize {
    front
        .yaml
        .lines()
        .position(|line| {
            line.strip_prefix(key)
                .is_some_and(|rest| rest.starts_with(':'))
        })
        .map_or(front.first_line, |index| front.first_line + index)
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
    reached: &[String],
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
            reached,
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

/// Every mechanism the plan's `touches` reach, or a changed file it owes
/// reaches, that no crossing answers. The declared reach is named first: it
/// is the line the author can fix without reading the diff.
fn uncrossed(
    front: &Frontmatter,
    touched: &Touched,
    reached: &[String],
    known: &[Mechanism],
    answered: &BTreeSet<String>,
    out: &mut Problems,
) {
    for mechanism in known.iter().filter(|m| !answered.contains(&m.id)) {
        let how = touched
            .reaches(mechanism)
            .map(|path| format!("the plan touches `{path}`"))
            .or_else(|| {
                let lives = scan::globs(&mechanism.paths).ok()?;
                reached
                    .iter()
                    .find(|path| lives.is_match(path))
                    .map(|path| format!("the work changed `{path}`"))
            });
        if let Some(how) = how {
            out.push(
                &format!("uncrossed:{}", mechanism.id),
                front.first_line,
                format!(
                    "{how}, where mechanism `{}` lives, and the plan never says what happens to it",
                    mechanism.id
                ),
                format!(
                    "a `crosses` entry for `{}`: works, refused or n/a",
                    mechanism.id
                ),
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
        if !is_glob(pattern) {
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
            judge(text, &opts, &files, Some(&catalog), None, &mut problems);
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
        assert_eq!(keys("plans/drifted.md"), Vec::<String>::new());
    }

    /// The same fixture read against a change: `plans/drifted.md` is in
    /// flight, and the change also edits a sessions file it never declared.
    /// The repair names the file as a deviation and answers the mechanism.
    #[test]
    fn the_fixture_trips_the_diff_refusals_and_the_repair_clears_them() {
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
        let opts = with_defaults(&dedented);
        let file = |rel: &str| {
            fixture
                .files
                .iter()
                .find(|(path, _)| *path == rel)
                .map(|(_, text)| text.to_string())
                .expect("the file is in the fixture")
        };
        let catalog = parse_mechanisms(&file("docs/mechanisms.yaml"), "docs/mechanisms.yaml")
            .expect("the catalog is valid");
        let changed = [
            "plans/drifted.md",
            "src/ledger/post.py",
            "src/sessions/guest.py",
            ".software-factory/policy.yaml",
        ];
        let drifted = file("plans/drifted.md");
        assert_eq!(
            diff_keys(
                &[("plans/drifted.md", &drifted)],
                &changed,
                &opts,
                Some(&catalog)
            ),
            vec![
                (
                    "plans/drifted.md".to_string(),
                    "untouched:src/sessions/guest.py".to_string()
                ),
                (
                    "plans/drifted.md".to_string(),
                    "uncrossed:sessions".to_string()
                ),
            ]
        );
        let repaired = drifted.replace(
            "exit: refunds post",
            "  - { mechanism: sessions, answer: n/a, why: guests never refund }\ndeviations:\n  - \"src/sessions/guest.py now opens the refund\"\nexit: refunds post",
        );
        assert_eq!(
            diff_keys(
                &[("plans/drifted.md", &repaired)],
                &changed,
                &opts,
                Some(&catalog)
            ),
            Vec::<(String, String)>::new()
        );
    }

    /// Options as the rule runs them: the instance merged over its default.
    fn with_defaults(instance: &str) -> Options {
        let catalog = crate::catalog::Catalog::builtin().expect("the shipped catalog loads");
        let defaults = &catalog
            .get("L4.PLAN_IS_A_LINKED_DECLARATION")
            .expect("the rule ships")
            .defaults;
        let instance: Value = serde_yaml::from_str(instance).expect("the options parse");
        serde_yaml::from_value(crate::policy::merge(defaults, &instance)).expect("they merge")
    }

    const MECHANISMS: &str = "mechanisms:\n  - { id: ledger, paths: [\"src/ledger/**\"] }\n  - { id: sessions, paths: [\"src/sessions/**\"] }\n  - { id: factory, paths: [\".software-factory/**\"] }\n";

    /// A linked plan answering `ledger`, with its `touches` and any extra
    /// top-level lines.
    fn plan(touches: &str, extra: &str) -> String {
        format!(
            "---\nsources:\n  - {{ id: S1, kind: decision }}\ntouches: {touches}\ncriteria:\n  - {{ id: A1, statement: refunds post, proof: \"test:tests/refund.py\", sources: [S1] }}\ncrosses:\n  - {{ mechanism: ledger, answer: works, proof: \"test:tests/refund.py\" }}\n{extra}exit: refunds post\n---\n# Plan\n"
        )
    }

    /// Every finding the plans earn against a change, as (plan, key) in the
    /// order the rule reports them.
    fn diff_keys(
        plans: &[(&str, &str)],
        changed: &[&str],
        opts: &Options,
        catalog: Option<&[Mechanism]>,
    ) -> Vec<(String, String)> {
        let plans: Vec<(String, String)> = plans
            .iter()
            .map(|(rel, text)| (rel.to_string(), text.to_string()))
            .collect();
        let changed: Vec<String> = changed.iter().map(|path| path.to_string()).collect();
        let files: Vec<scan::SourceFile> = plans
            .iter()
            .map(|(rel, _)| rel.clone())
            .chain(changed.iter().cloned())
            .map(|rel| scan::SourceFile {
                abs: std::path::PathBuf::from(&rel),
                rel,
            })
            .collect();
        judge_plans(&plans, Some(&changed), opts, &files, catalog)
            .expect("the plans are judged")
            .into_iter()
            .flat_map(|problems| {
                let rel = problems.rel;
                problems
                    .found
                    .into_iter()
                    .map(move |(key, ..)| (rel.clone(), key))
            })
            .collect()
    }

    fn of(plan: &str, keys: &[&str]) -> Vec<(String, String)> {
        keys.iter()
            .map(|key| (plan.to_string(), key.to_string()))
            .collect()
    }

    #[test]
    fn a_changed_file_outside_every_touches_is_untouched_and_one_inside_is_not() {
        let opts = with_defaults("{}");
        let catalog = parse_mechanisms(MECHANISMS, "m.yaml").expect("valid");
        let p = plan("[\"src/ledger/**\"]", "");
        let changed = ["plans/p.md", "src/ledger/post.py", "src/billing/charge.py"];
        assert_eq!(
            diff_keys(&[("plans/p.md", &p)], &changed, &opts, Some(&catalog)),
            of("plans/p.md", &["untouched:src/billing/charge.py"])
        );
        let declared = plan("[\"src/ledger/**\", \"src/billing/charge.py\"]", "");
        assert_eq!(
            diff_keys(
                &[("plans/p.md", &declared)],
                &changed,
                &opts,
                Some(&catalog)
            ),
            Vec::new(),
            "a literal touch declares exactly its path"
        );
    }

    #[test]
    fn an_incidental_file_needs_no_declaration_and_reaches_no_mechanism() {
        let catalog = parse_mechanisms(MECHANISMS, "m.yaml").expect("valid");
        let p = plan("[\"src/ledger/**\"]", "");
        let other = plan("[]", "");
        let changed = [
            "plans/p.md",
            "plans/other.md",
            "plans/delivered-and-gone.md",
            "plans/next-steps.md",
            ".software-factory/policy.yaml",
            "src/ledger/post.py",
            "CHANGELOG.md",
        ];
        let plans = [
            ("plans/p.md", p.as_str()),
            ("plans/other.md", other.as_str()),
        ];
        assert_eq!(
            diff_keys(&plans, &changed, &with_defaults("{}"), Some(&catalog)),
            vec![
                (
                    "plans/other.md".to_string(),
                    "untouched:CHANGELOG.md".to_string()
                ),
                (
                    "plans/p.md".to_string(),
                    "untouched:CHANGELOG.md".to_string()
                ),
            ],
            "the factory directory, the board, a removed plan and every in-flight plan are incidental; `factory` is never uncrossed"
        );
        assert_eq!(
            diff_keys(
                &plans,
                &changed,
                &with_defaults(
                    "incidental: [\".software-factory/**\", \"plans/next-steps.md\", \"*.md\"]"
                ),
                Some(&catalog),
            ),
            Vec::new()
        );
    }

    #[test]
    fn a_file_named_under_deviations_is_accounted_for_and_a_near_miss_is_not() {
        let opts = with_defaults("{}");
        let changed = ["plans/p.md", "src/billing/charge.py"];
        let named = plan(
            "[]",
            "deviations:\n  - \"the charge moved into `src/billing/charge.py`.\"\n",
        );
        assert_eq!(
            diff_keys(&[("plans/p.md", &named)], &changed, &opts, None),
            Vec::new()
        );
        for near in [
            "src/billing/charge.py.bak",
            "src/billing/charge.py~",
            "src/billing/charge.py+old",
            "lib/src/billing/charge.py",
            "src/billing/charge.pyc",
        ] {
            let missed = plan(
                "[]",
                &format!("deviations:\n  - \"kept {near} as it was\"\n"),
            );
            assert_eq!(
                diff_keys(&[("plans/p.md", &missed)], &changed, &opts, None),
                of("plans/p.md", &["untouched:src/billing/charge.py"]),
                "{near} is not the changed path"
            );
        }
    }

    #[test]
    fn a_plan_not_in_flight_is_not_read_against_the_diff() {
        let opts = with_defaults("{}");
        let catalog = parse_mechanisms(MECHANISMS, "m.yaml").expect("valid");
        let old = plan("[\"src/ledger/post.py\"]", "");
        let changed = ["src/sessions/guest.py", "src/billing/charge.py"];
        assert_eq!(
            diff_keys(&[("plans/old.md", &old)], &changed, &opts, Some(&catalog)),
            Vec::new(),
            "its own file is not in the change, so nothing it says is held to it"
        );
    }

    #[test]
    fn a_mechanism_reached_only_through_the_diff_is_uncrossed() {
        let opts = with_defaults("{}");
        let catalog = parse_mechanisms(MECHANISMS, "m.yaml").expect("valid");
        let deviated = plan(
            "[\"src/ledger/**\"]",
            "deviations:\n  - \"src/sessions/guest.py had to open the refund\"\n",
        );
        let changed = ["plans/p.md", "src/ledger/post.py", "src/sessions/guest.py"];
        assert_eq!(
            diff_keys(
                &[("plans/p.md", &deviated)],
                &changed,
                &opts,
                Some(&catalog)
            ),
            of("plans/p.md", &["uncrossed:sessions"]),
            "a deviation accounts for the file, not for the mechanism it reaches"
        );
        let undeclared = plan("[\"src/ledger/**\"]", "");
        assert_eq!(
            diff_keys(
                &[("plans/p.md", &undeclared)],
                &changed,
                &opts,
                Some(&catalog)
            ),
            of(
                "plans/p.md",
                &["untouched:src/sessions/guest.py", "uncrossed:sessions"]
            )
        );
        let mut problems = Problems {
            rel: "plans/p.md".into(),
            found: Vec::new(),
        };
        let owed = Owed {
            untouched: Vec::new(),
            reached: vec!["src/sessions/guest.py".into()],
        };
        judge(
            &deviated,
            &opts,
            &[],
            Some(&catalog),
            Some(&owed),
            &mut problems,
        );
        assert!(
            problems.found[0]
                .2
                .contains("the work changed `src/sessions/guest.py`"),
            "the message says the reach came from the diff: {:?}",
            problems.found
        );
    }

    #[test]
    fn a_file_another_plan_declares_is_that_plan_s_crossing() {
        let opts = with_defaults("{}");
        let catalog = parse_mechanisms(MECHANISMS, "m.yaml").expect("valid");
        let owner = plan("[\"src/ledger/**\", \"src/sessions/**\"]", "");
        let bystander = plan("[\"src/ledger/**\"]", "");
        let changed = ["plans/a.md", "plans/b.md", "src/sessions/guest.py"];
        assert_eq!(
            diff_keys(
                &[("plans/a.md", &owner), ("plans/b.md", &bystander)],
                &changed,
                &opts,
                Some(&catalog),
            ),
            of("plans/a.md", &["uncrossed:sessions"])
        );
    }

    #[test]
    fn the_diff_findings_are_sorted_and_the_same_every_run() {
        let opts = with_defaults("{}");
        let catalog = parse_mechanisms(MECHANISMS, "m.yaml").expect("valid");
        let p = plan("[]", "");
        let changed = [
            "src/z.py",
            "plans/p.md",
            "src/a.py",
            "src/sessions/guest.py",
            "src/a.py",
            "src/m.py",
        ];
        let first = diff_keys(&[("plans/p.md", &p)], &changed, &opts, Some(&catalog));
        assert_eq!(
            first,
            of(
                "plans/p.md",
                &[
                    "untouched:src/a.py",
                    "untouched:src/m.py",
                    "untouched:src/sessions/guest.py",
                    "untouched:src/z.py",
                    "uncrossed:sessions",
                ]
            )
        );
        let mut reversed = changed;
        reversed.reverse();
        assert_eq!(
            diff_keys(&[("plans/p.md", &p)], &reversed, &opts, Some(&catalog)),
            first
        );
    }

    /// A scratch directory under the system temp dir, unique per test.
    fn scratch(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let dir =
            std::env::temp_dir().join(format!("sf-diff-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("the scratch directory is created");
        dir
    }

    /// Git in `dir` alone, whatever repository a hook running these tests
    /// exported.
    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=sf",
                "-c",
                "user.email=sf@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?} failed");
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the parent is created");
        std::fs::write(path, text).expect("the file is written");
    }

    #[test]
    fn changed_wins_then_the_default_branch_then_nothing() {
        let dir = scratch("precedence");
        let given = vec![
            "src/b.py".to_string(),
            "src/a.py".to_string(),
            "src/a.py".to_string(),
        ];
        assert_eq!(
            work_changed(&dir, Some(&given), Some("no-such-branch")),
            Ok(Some(vec!["src/a.py".to_string(), "src/b.py".to_string()])),
            "--changed is read as given, sorted, and git is never asked"
        );
        assert_eq!(work_changed(&dir, None, None), Ok(None));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_default_branch_reads_the_tree_against_its_merge_base() {
        let dir = scratch("merge-base");
        run_git(&dir, &["init", "-q", "-b", "main"]);
        write(&dir, "src/ledger/post.py", "def post(): pass\n");
        write(&dir, "src/kept.py", "x = 1\n");
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "base"]);
        run_git(&dir, &["checkout", "-q", "-b", "work"]);
        write(&dir, "src/ledger/post.py", "def post(): return 1\n");
        run_git(&dir, &["commit", "-qam", "committed on the branch"]);
        // main moves on after the branch point: its change is not the work's.
        run_git(&dir, &["checkout", "-q", "main"]);
        write(&dir, "src/kept.py", "x = 2\n");
        run_git(&dir, &["commit", "-qam", "main moved"]);
        run_git(&dir, &["checkout", "-q", "work"]);
        write(&dir, "src/odd name ü.py", "y = 1\n");
        run_git(&dir, &["add", "src/odd name ü.py"]);
        write(&dir, "src/sessions/guest.py", "untracked = True\n");
        assert_eq!(
            work_changed(&dir, None, Some("main")),
            Ok(Some(vec![
                "src/ledger/post.py".to_string(),
                "src/odd name ü.py".to_string(),
            ])),
            "committed and staged work since the merge-base, paths exact; main's own commit and untracked files are not in it"
        );
        assert!(
            work_changed(&dir, None, Some("no-such-branch"))
                .err()
                .is_some_and(|why| why.contains("git merge-base HEAD no-such-branch")),
            "a branch git cannot resolve is an error, not an empty change"
        );

        // The whole rule over that repository: the plan delivered into
        // docs/design in this change is in flight, and owes the sessions
        // file it never declared.
        write(
            &dir,
            ".software-factory/policy.yaml",
            "version: 1\nproject:\n  name: t\n  languages: []\n  default_branch: main\nrules:\n  L4.PLAN_IS_A_LINKED_DECLARATION:\n    enabled: true\n",
        );
        write(
            &dir,
            "docs/design/delivered.md",
            &plan("[\"src/ledger/**\", \"src/odd name ü.py\"]", ""),
        );
        write(&dir, "docs/design/older.md", &plan("[]", ""));
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-qm", "deliver"]);
        let older = std::fs::read_to_string(dir.join("docs/design/older.md")).expect("reads");
        run_git(&dir, &["checkout", "-q", "main"]);
        write(&dir, "docs/design/older.md", &older);
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-qm", "an older delivered plan"]);
        run_git(&dir, &["checkout", "-q", "work"]);
        run_git(&dir, &["merge", "-q", "--no-edit", "main"]);
        write(&dir, "src/sessions/guest.py", "tracked = True\n");
        run_git(&dir, &["add", "src/sessions/guest.py"]);
        let keys: Vec<String> = rule_over(&dir)
            .into_iter()
            .map(|finding| finding.key)
            .collect();
        assert_eq!(
            keys,
            vec!["untouched:src/sessions/guest.py:docs/design/delivered.md".to_string()],
            "the delivered plan is in flight; the older one, unchanged since main, is not"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_git_failure_is_reported_rather_than_read_as_no_change() {
        let dir = scratch("not-a-repo");
        write(
            &dir,
            ".software-factory/policy.yaml",
            "version: 1\nproject:\n  name: t\n  languages: []\n  default_branch: main\nrules:\n  L4.PLAN_IS_A_LINKED_DECLARATION:\n    enabled: true\n",
        );
        write(&dir, "plans/p.md", &plan("[]", ""));
        let findings = rule_over(&dir);
        let failure = findings
            .iter()
            .find(|finding| finding.key == "changed-set")
            .expect("the git failure is a finding");
        assert!(
            failure.message.contains("git merge-base HEAD main"),
            "it names the command: {}",
            failure.message
        );
        assert!(!findings.iter().any(|f| f.key.starts_with("untouched:")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `L4.PLAN_IS_A_LINKED_DECLARATION` over a repository on disk, as
    /// `sf check` runs it with no `--changed`.
    fn rule_over(dir: &Path) -> Vec<Finding> {
        rule_over_changed(dir, None)
    }

    /// The same, with the changed set `--changed` handed the run.
    fn rule_over_changed(dir: &Path, changed: Option<Vec<String>>) -> Vec<Finding> {
        let policy = crate::policy::Policy::load(dir).expect("the policy loads");
        let catalog = crate::catalog::Catalog::builtin().expect("the catalog loads");
        let files = scan::walk(dir, &policy).expect("the repository scans");
        let ratchet = crate::ratchet::Ratchet::default();
        let ctx = Ctx {
            root: dir,
            policy: &policy,
            catalog: &catalog,
            files: &files,
            ratchet: &ratchet,
            changed,
            base: None,
            today: crate::clock::today(),
            allow_commands: false,
            overlay: None,
        };
        let rule = catalog
            .get("L4.PLAN_IS_A_LINKED_DECLARATION")
            .expect("the rule ships");
        let opts = crate::checks::options_for(rule, &policy).expect("the options merge");
        plan_declaration(rule, &opts, &ctx).expect("the rule runs")
    }

    #[test]
    fn changed_reads_a_non_ascii_path_as_the_path_itself() {
        let dir = scratch("changed-non-ascii");
        run_git(&dir, &["init", "-q", "-b", "main"]);
        write(
            &dir,
            ".software-factory/policy.yaml",
            "version: 1\nproject:\n  name: t\n  languages: []\nrules:\n  L4.PLAN_IS_A_LINKED_DECLARATION:\n    enabled: true\n",
        );
        write(&dir, "src/módulo.py", "x = 1\n");
        run_git(&dir, &["add", "-A"]);
        run_git(&dir, &["commit", "-q", "-m", "base"]);
        write(&dir, "plans/ação.md", &plan("[\"src/módulo.py\"]", ""));
        write(&dir, "src/módulo.py", "x = 2\n");
        run_git(&dir, &["add", "-A"]);
        // Through the producer `sf check --changed main` uses, not a list
        // typed in: line mode would hand back `"plans/a\303\247\303\243o.md"`.
        let changed = crate::changed_paths(&dir, "main").expect("git reads the change");
        assert_eq!(
            changed,
            vec!["plans/ação.md".to_string(), "src/módulo.py".to_string()],
            "the paths arrive exact, not git-quoted"
        );
        let keys: Vec<String> = rule_over_changed(&dir, Some(changed))
            .into_iter()
            .map(|finding| finding.key)
            .collect();
        assert!(
            !keys.iter().any(|key| key.starts_with("untouched:")),
            "the non-ASCII plan is in flight and declares the non-ASCII source: {keys:?}"
        );
        // Without the plan in the change, the source is the change's own:
        // the plan is not in flight, so nothing is read against the diff.
        let only_source = vec!["src/módulo.py".to_string()];
        assert!(
            rule_over_changed(&dir, Some(only_source))
                .iter()
                .all(|finding| !finding.key.starts_with("untouched:")),
            "a plan not in flight owes nothing"
        );
        assert!(
            crate::changed_paths(&dir, "no-such-ref")
                .err()
                .is_some_and(|why| why
                    .to_string()
                    .contains("git diff --name-only -z no-such-ref")),
            "a reference git cannot resolve is an error naming the command, never an empty change"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unparsable_delivered_note_is_refused_and_its_work_does_not_pass() {
        let dir = scratch("unparsable-delivered");
        write(
            &dir,
            ".software-factory/policy.yaml",
            "version: 1\nproject:\n  name: t\n  languages: []\nrules:\n  L4.PLAN_IS_A_LINKED_DECLARATION:\n    enabled: true\n",
        );
        write(
            &dir,
            "docs/design/broken.md",
            "---\ntouches: [\"src/ledger/**\"\ncriteria: [\n---\n# Broken\n",
        );
        write(&dir, "src/ledger/post.py", "def post(): pass\n");
        let changed = vec![
            "docs/design/broken.md".to_string(),
            "src/ledger/post.py".to_string(),
        ];
        let keys: Vec<String> = rule_over_changed(&dir, Some(changed))
            .into_iter()
            .map(|finding| finding.key)
            .collect();
        assert_eq!(
            keys,
            vec![
                "untouched:src/ledger/post.py:docs/design/broken.md".to_string(),
                "unparsable:docs/design/broken.md".to_string(),
            ],
            "the delivered note is in flight though it does not parse: it is refused, and the source it carried is owed rather than passed"
        );
        let _ = std::fs::remove_dir_all(&dir);
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
        judge(text, &opts, &[], None, None, &mut problems);
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

/// The coverage relation over every change rather than a handful: a changed
/// path is reported exactly when no in-flight plan's `touches` declares it,
/// no `incidental` glob matches it and no deviation names it, whatever near
/// misses sit beside it.
///
/// Its falsifier is `a_prefix_covers`: a matcher that lets a literal touch
/// declare every path it is a prefix of, so `src/a` declares `src/ab.py`.
/// Planted when `SF_FALSIFIER` names it, which is how
/// `L5.EVERY_PROPERTY_HAS_A_FALSIFIER` proves this property can fail.
#[cfg(test)]
mod property {
    use super::{Covers, Flight, Work, touches_cover};
    use proptest::prelude::*;
    use proptest::test_runner::Config;

    fn a_prefix_covers(touches: &[String], path: &str) -> bool {
        touches_cover(touches, path) || touches.iter().any(|t| path.starts_with(t.as_str()))
    }

    /// The matcher under test: the real one, unless a falsifier is planted.
    fn covers() -> Covers {
        match std::env::var(crate::checks::properties::FALSIFIER_VAR).as_deref() {
            Ok("a_prefix_covers") => a_prefix_covers,
            _ => touches_cover,
        }
    }

    /// How one changed path is, or is not, accounted for.
    #[derive(Debug, Clone, Copy)]
    enum Role {
        Declared,
        Incidental,
        Deviated,
        Uncovered,
    }

    /// Short names over a small alphabet, so paths share prefixes and
    /// directories often: the near misses are the cases worth generating.
    fn path() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::sample::select(vec!["a", "b", "ab", "a.py", "ab.py"]),
            1..4,
        )
        .prop_map(|parts| parts.join("/"))
    }

    fn change() -> impl Strategy<Value = Vec<(String, Role, usize)>> {
        prop::collection::btree_map(
            path(),
            (
                prop_oneof![
                    Just(Role::Declared),
                    Just(Role::Incidental),
                    Just(Role::Deviated),
                    Just(Role::Uncovered),
                ],
                0usize..2,
            ),
            1..10,
        )
        .prop_map(|paths| {
            paths
                .into_iter()
                .map(|(path, (role, flight))| (path, role, flight))
                .collect()
        })
    }

    proptest! {
        // No regression file: a falsifier run is meant to fail, and must not
        // write into the tree it is checking.
        #![proptest_config(Config { cases: 512, failure_persistence: None, ..Config::default() })]

        #[test]
        fn a_changed_path_is_reported_exactly_when_nothing_accounts_for_it(change in change()) {
            let changed: Vec<String> = change.iter().map(|(path, ..)| path.clone()).collect();
            let mut flights = vec![
                Flight { rel: "plans/one.md".into(), touches: Vec::new(), deviations: Vec::new() },
                Flight { rel: "plans/two.md".into(), touches: Vec::new(), deviations: Vec::new() },
            ];
            let mut incidental = Vec::new();
            let mut expected = Vec::new();
            for (path, role, flight) in &change {
                let flight = &mut flights[*flight];
                match role {
                    Role::Declared => flight.touches.push(path.clone()),
                    Role::Incidental => incidental.push(path.clone()),
                    Role::Deviated => flight.deviations.push(format!("moved `{path}` on review")),
                    Role::Uncovered => {
                        expected.push(path.clone());
                        // Near misses that must not count: a strict prefix,
                        // a longer sibling, both as touch and as mention.
                        for near in [&path[..path.len() - 1], &format!("{path}x")] {
                            if !near.is_empty() && !changed.iter().any(|c| c == near) {
                                flight.touches.push(near.to_string());
                                flight.deviations.push(format!("kept {near} as it was"));
                            }
                        }
                    }
                }
            }
            let work = Work {
                changed,
                flights,
                incidental: crate::scan::globs(&incidental).expect("literal globs compile"),
                plans: globset::GlobSet::empty(),
                plans_excluded: globset::GlobSet::empty(),
            };
            prop_assert_eq!(work.uncovered(covers()), expected);
        }
    }
}
