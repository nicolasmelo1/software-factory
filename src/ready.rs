//! `sf ready`: whether a plan still matches the code, asked once, before work
//! on it starts. Intent changes when somebody decides; facts about the code go
//! stale with every merge. So facts are pinned to `base:` and asked again:
//! what is gone, what moved, what changed since `base` in the files cited.
//! Not a pull request gate: a plan may drift until somebody is about to build it.

use crate::checks::plan_declaration::declaration;
use anyhow::{Context, Result, bail};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// What `sf ready` found, split by what clears it.
#[derive(Debug, Default)]
pub struct Readiness {
    /// A fact that no longer holds: fix the plan's line.
    pub broken: Vec<String>,
    /// A file the plan cites changed since `base`: re-read, then stamp.
    pub drifted: Vec<String>,
    /// Said for the reader; clears nothing and blocks nothing.
    pub notes: Vec<String>,
}

impl Readiness {
    pub fn ready(&self) -> bool {
        self.broken.is_empty() && self.drifted.is_empty()
    }
}

/// Judge `plan` (repo-relative) against HEAD.
pub fn judge(root: &Path, plan: &str) -> Result<Readiness> {
    let text =
        std::fs::read_to_string(root.join(plan)).with_context(|| format!("reading {plan}"))?;
    let map = match declaration(&text) {
        Some(Ok((_, map))) => map,
        Some(Err(why)) => bail!("{plan}: {why}"),
        None => bail!("{plan} is prose with no declaration; there are no pinned facts to check"),
    };
    let git = Git { root };
    let base = string(map.get("base"));
    let mut out = Readiness::default();
    let mut cited: BTreeMap<String, Vec<String>> = BTreeMap::new();

    judge_sources(&git, base.as_deref(), &map, &mut out, &mut cited);
    for touched in strings(map.get("touches")) {
        judge_touch(&git, base.as_deref(), &touched, &mut out, &mut cited);
    }
    for (key, name) in [("criteria", "id"), ("crosses", "mechanism")] {
        for entry in sequence(&map, key) {
            let id = string(entry.get(name)).unwrap_or_default();
            for path in tests_in(&string(entry.get("proof")).unwrap_or_default()) {
                judge_test(&git, base.as_deref(), &id, &path, &mut out);
            }
        }
    }
    judge_base(&git, base.as_deref(), &cited, &mut out);
    Ok(out)
}

fn judge_sources(
    git: &Git,
    base: Option<&str>,
    map: &Mapping,
    out: &mut Readiness,
    cited: &mut BTreeMap<String, Vec<String>>,
) {
    for source in sequence(map, "sources") {
        let id = string(source.get("id")).unwrap_or_else(|| "a source".to_string());
        if let Some(said) = string(source.get("said")) {
            judge_said(&id, &said, out);
        }
        if let Some(reference) = string(source.get("ref")) {
            judge_reference(git, base, &id, &reference, out, cited);
            judge_symbols(git, &id, &reference, source.get("symbol"), out);
        }
    }
}

/// Pin the plan's facts to HEAD by writing `base:`, replacing the one it had.
pub fn stamp(root: &Path, plan: &str) -> Result<String> {
    let head = Git { root }.head()?;
    let path = root.join(plan);
    let text = std::fs::read_to_string(&path)?;
    let mut lines: Vec<&str> = text.split_inclusive('\n').collect();
    let close = lines
        .iter()
        .skip(1)
        .position(|line| line.trim_end() == "---")
        .map(|i| i + 1)
        .context("the plan's declaration is not closed")?;
    let line = format!("base: {head}\n");
    let key = regex::Regex::new(r#"^["']?base["']?\s*:"#).expect("a fixed pattern compiles");
    match lines[1..close].iter().position(|l| key.is_match(l)) {
        Some(at) => lines[at + 1] = &line,
        None => lines.insert(1, &line),
    }
    std::fs::write(&path, lines.concat())?;
    Ok(head)
}

fn judge_reference(
    git: &Git,
    base: Option<&str>,
    id: &str,
    reference: &str,
    out: &mut Readiness,
    cited: &mut BTreeMap<String, Vec<String>>,
) {
    if reference.contains("://") || reference.starts_with('#') || reference.starts_with("issue") {
        return;
    }
    if let Some((rev, path)) = reference.split_once(':')
        && !path.is_empty()
        && !path
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == ',')
    {
        if !git.exists(rev, path) {
            out.broken
                .push(format!("{id} cites {reference}, which git cannot read"));
        }
        return;
    }
    let (path, line) = match reference.split_once(':') {
        Some((path, line)) => (path, Some(line)),
        None => (reference, None),
    };
    if line.is_some() {
        out.broken.push(format!(
            "{id} points at {reference}: a line number goes stale with the next edit above it; name the symbol in `symbol:` instead"
        ));
    }
    if git.exists("HEAD", path) {
        cited
            .entry(path.to_string())
            .or_default()
            .push(id.to_string());
    } else {
        out.broken.push(gone(id, path, base, git));
    }
}

/// The `test:` paths a proof names, each apart from the annotation beside it.
/// A `deferred:` or `unspecified:` proof names a test the work will write.
fn tests_in(proof: &str) -> Vec<String> {
    let proof = proof.trim();
    if proof.starts_with("deferred:") || proof.starts_with("unspecified:") {
        return Vec::new();
    }
    let test =
        regex::Regex::new(r"(?:^|[\s,;(])test:([^\s,;)]+)").expect("a fixed pattern compiles");
    test.captures_iter(proof)
        .map(|c| c[1].to_string())
        .collect()
}

/// Where a `ref` lives: a `<rev>:<path>` into history, or a path at HEAD.
fn located(reference: &str) -> (&str, &str) {
    match reference.split_once(':') {
        Some((rev, path))
            if !path.is_empty()
                && !path
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '-' || c == ',') =>
        {
            (rev, path)
        }
        Some((path, _line)) => ("HEAD", path),
        None => ("HEAD", reference),
    }
}

/// A line number written into a sentence goes stale exactly like one in
/// `ref`, and is harder to see.
fn judge_said(id: &str, said: &str, out: &mut Readiness) {
    let line =
        regex::Regex::new(r"[\w./-]+\.[A-Za-z]+:\d+(?:-\d+)?").expect("a fixed pattern compiles");
    for found in line.find_iter(said) {
        out.broken.push(format!(
            "{id} says {}: a line number goes stale with the next edit above it; name the symbol instead",
            found.as_str()
        ));
    }
}

fn judge_symbols(
    git: &Git,
    id: &str,
    reference: &str,
    symbols: Option<&Value>,
    out: &mut Readiness,
) {
    let (rev, path) = located(reference);
    let Some(code) = git.read(rev, path) else {
        return;
    };
    for symbol in strings(symbols) {
        let found = code.match_indices(symbol.as_str()).any(|(at, _)| {
            let before = code[..at].chars().next_back();
            let after = code[at + symbol.len()..].chars().next();
            !before.is_some_and(is_word) && !after.is_some_and(is_word)
        });
        if !found {
            out.broken.push(format!(
                "{id} names `{symbol}`, which {path} at {rev} does not have"
            ));
        }
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn judge_touch(
    git: &Git,
    base: Option<&str>,
    touched: &str,
    out: &mut Readiness,
    cited: &mut BTreeMap<String, Vec<String>>,
) {
    let path = touched.split_whitespace().next().unwrap_or(touched);
    if path.contains(['*', '?', '[', '{']) {
        cited
            .entry(path.to_string())
            .or_default()
            .push("touches".to_string());
        return;
    }
    if git.exists("HEAD", path) {
        cited
            .entry(path.to_string())
            .or_default()
            .push("touches".to_string());
    } else if base.is_some_and(|base| git.exists(base, path)) {
        out.broken.push(gone("touches", path, base, git));
    } else {
        out.notes
            .push(format!("touches {path}, which the work creates"));
    }
}

fn judge_test(git: &Git, base: Option<&str>, id: &str, path: &str, out: &mut Readiness) {
    if git.exists("HEAD", path) {
        return;
    }
    if base.is_some_and(|base| git.exists(base, path)) {
        out.broken.push(gone(id, path, base, git));
    } else {
        out.notes
            .push(format!("{id} is proven by {path}, a test the work creates"));
    }
}

fn judge_base(
    git: &Git,
    base: Option<&str>,
    cited: &BTreeMap<String, Vec<String>>,
    out: &mut Readiness,
) {
    let Some(base) = base else {
        out.drifted.push(
            "no `base:` — nothing says which commit these facts were read at; re-read them, then stamp".to_string(),
        );
        return;
    };
    if !git.is_commit(base) {
        out.broken
            .push(format!("base {base} is not a commit this repository has"));
        return;
    }
    for path in git.changed_since(base) {
        let by: Vec<&String> = cited
            .iter()
            .filter(|(cite, _)| covers(cite, &path))
            .flat_map(|(_, ids)| ids)
            .collect();
        if by.is_empty() {
            continue;
        }
        out.drifted.push(format!(
            "{path} changed since base {}; re-read what cites it: {}",
            &base[..base.len().min(12)],
            by.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
}

/// Whether a cited path, directory or glob covers a changed file.
fn covers(cite: &str, path: &str) -> bool {
    if cite.contains(['*', '?', '[', '{']) {
        return crate::scan::globs(&[cite.to_string()]).is_ok_and(|set| set.is_match(path));
    }
    path == cite || path.starts_with(&format!("{}/", cite.trim_end_matches('/')))
}

fn gone(id: &str, path: &str, base: Option<&str>, git: &Git) -> String {
    match base {
        Some(base) if git.exists(base, path) => {
            format!("{id} cites {path}, which existed at base and is gone: moved or deleted since")
        }
        _ => format!("{id} cites {path}, which does not exist"),
    }
}

struct Git<'a> {
    root: &'a Path,
}

impl Git<'_> {
    fn run(&self, args: &[&str]) -> Option<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(args)
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn head(&self) -> Result<String> {
        self.run(&["rev-parse", "HEAD"])
            .map(|s| s.trim().to_string())
            .context("`git rev-parse HEAD` failed: is this a git repository with a commit?")
    }

    fn is_commit(&self, rev: &str) -> bool {
        self.run(&["cat-file", "-e", &format!("{rev}^{{commit}}")])
            .is_some()
    }

    fn exists(&self, rev: &str, path: &str) -> bool {
        self.run(&["cat-file", "-e", &format!("{rev}:{path}")])
            .is_some()
    }

    fn read(&self, rev: &str, path: &str) -> Option<String> {
        self.run(&["show", &format!("{rev}:{path}")])
    }

    /// Every file changed between `base` and HEAD, unfiltered: the plan's
    /// own glob matcher selects from it, so a brace set means here what it
    /// means everywhere else a declaration is read.
    fn changed_since(&self, base: &str) -> Vec<String> {
        self.run(&["diff", "--name-only", base, "HEAD"])
            .map(|out| out.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }
}

fn string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn strings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Sequence(items)) => items.iter().filter_map(|v| string(Some(v))).collect(),
        Some(other) => string(Some(other)).into_iter().collect(),
        None => Vec::new(),
    }
}

fn sequence(map: &Mapping, key: &str) -> Vec<Value> {
    match map.get(key) {
        Some(Value::Sequence(items)) => items.clone(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let root =
            std::env::temp_dir().join(format!("sf-ready-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).expect("the scratch repository is created");
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "fixture@example.com"],
            vec!["config", "user.name", "fixture"],
        ] {
            git(&root, &args);
        }
        root
    }

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?} succeeds");
    }

    fn commit(root: &Path, files: &[(&str, &str)]) {
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().expect("a file has a parent"))
                .expect("its directory exists");
            std::fs::write(path, text).expect("the file is written");
        }
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "change"]);
    }

    const PLAN: &str = "---\nsources:\n  - { id: S1, kind: code, ref: src/ledger.rs, symbol: post }\ntouches: [src/ledger.rs, src/refund.rs]\ncriteria:\n  - { id: A1, statement: refunds post, proof: \"test:tests/refund.rs\", sources: [S1] }\nexit: refunds post\n---\n# Refunds\n";

    #[test]
    fn a_plan_with_no_base_is_not_ready_until_it_is_stamped() {
        let root = repo("unstamped");
        commit(
            &root,
            &[("src/ledger.rs", "pub fn post() {}\n"), ("plan.md", PLAN)],
        );
        let before = judge(&root, "plan.md").expect("the plan is judged");
        assert!(!before.ready());
        assert!(before.broken.is_empty(), "{:?}", before.broken);
        assert!(before.drifted[0].contains("no `base:`"));
        assert!(
            before
                .notes
                .iter()
                .any(|n| n.contains("src/refund.rs, which the work creates"))
        );

        stamp(&root, "plan.md").expect("the plan is stamped");
        let after = judge(&root, "plan.md").expect("the plan is judged again");
        assert!(after.ready(), "{after:?}");
    }

    #[test]
    fn a_change_to_a_cited_file_since_base_names_the_source_to_re_read() {
        let root = repo("drifted");
        commit(
            &root,
            &[("src/ledger.rs", "pub fn post() {}\n"), ("plan.md", PLAN)],
        );
        stamp(&root, "plan.md").expect("the plan is stamped");
        commit(
            &root,
            &[("src/ledger.rs", "pub fn post() {}\npub fn void() {}\n")],
        );
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert_eq!(found.drifted.len(), 1, "{found:?}");
        assert!(found.drifted[0].contains("src/ledger.rs changed since base"));
        assert!(found.drifted[0].contains("S1"));
        assert!(found.drifted[0].contains("touches"));
    }

    #[test]
    fn a_symbol_or_a_file_that_is_gone_is_broken_not_drifted() {
        let root = repo("broken");
        commit(
            &root,
            &[("src/ledger.rs", "pub fn post() {}\n"), ("plan.md", PLAN)],
        );
        stamp(&root, "plan.md").expect("the plan is stamped");
        git(&root, &["rm", "-q", "src/ledger.rs"]);
        commit(&root, &[("src/books.rs", "pub fn record() {}\n")]);
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert!(
            found
                .broken
                .iter()
                .any(|b| b.contains("S1 cites src/ledger.rs, which existed at base and is gone")),
            "{found:?}"
        );
        assert!(
            found
                .broken
                .iter()
                .any(|b| b.contains("touches cites src/ledger.rs"))
        );
    }

    #[test]
    fn a_line_number_and_a_missing_symbol_are_refused() {
        let root = repo("lines");
        let plan = PLAN.replace(
            "ref: src/ledger.rs, symbol: post",
            "ref: \"src/ledger.rs:3\", symbol: [post, unpost]",
        );
        commit(
            &root,
            &[
                ("src/ledger.rs", "pub fn post() {}\npub fn poster() {}\n"),
                ("plan.md", plan.as_str()),
            ],
        );
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert!(
            found
                .broken
                .iter()
                .any(|b| b.contains("points at src/ledger.rs:3")),
            "{found:?}"
        );
        assert!(
            found.broken.iter().any(|b| b.contains("names `unpost`")),
            "{found:?}"
        );
        assert!(
            !found.broken.iter().any(|b| b.contains("names `post`")),
            "{found:?}"
        );
    }

    #[test]
    fn stamping_replaces_a_quoted_base_rather_than_adding_a_second() {
        let root = repo("quoted");
        let plan = PLAN.replacen("---\n", "---\n\"base\": 0000000\n", 1);
        commit(
            &root,
            &[
                ("src/ledger.rs", "pub fn post() {}\n"),
                ("plan.md", plan.as_str()),
            ],
        );
        stamp(&root, "plan.md").expect("the plan is stamped");
        let text = std::fs::read_to_string(root.join("plan.md")).expect("the plan reads");
        assert_eq!(text.matches("base").count(), 1, "{text}");
        assert!(judge(&root, "plan.md").expect("the plan is judged").ready());
    }

    #[test]
    fn a_crossing_or_annotated_proof_whose_test_is_gone_is_broken() {
        let root = repo("proofs");
        let plan = PLAN
            .replace("proof: \"test:tests/refund.rs\"", "proof: \"test:tests/refund.rs; the refund half\"")
            .replace("exit: refunds post", "crosses:\n  - { mechanism: ledger, answer: works, proof: \"test:tests/ledger.rs\" }\nexit: refunds post");
        commit(
            &root,
            &[
                ("src/ledger.rs", "pub fn post() {}\n"),
                ("tests/refund.rs", "#[test] fn r() {}\n"),
                ("tests/ledger.rs", "#[test] fn l() {}\n"),
                ("plan.md", plan.as_str()),
            ],
        );
        stamp(&root, "plan.md").expect("the plan is stamped");
        git(&root, &["rm", "-q", "tests/refund.rs", "tests/ledger.rs"]);
        git(&root, &["commit", "-q", "-m", "drop the tests"]);
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert!(
            found
                .broken
                .iter()
                .any(|b| b.starts_with("A1 cites tests/refund.rs, which existed at base")),
            "{found:?}"
        );
        assert!(
            found
                .broken
                .iter()
                .any(|b| b.starts_with("ledger cites tests/ledger.rs, which existed at base")),
            "{found:?}"
        );
    }

    #[test]
    fn facts_are_read_from_head_not_from_an_uncommitted_edit() {
        let root = repo("dirty");
        let plan = PLAN.replace("symbol: post", "symbol: [post, void]");
        commit(
            &root,
            &[
                ("src/ledger.rs", "pub fn post() {}\n"),
                ("plan.md", plan.as_str()),
            ],
        );
        std::fs::write(
            root.join("src/ledger.rs"),
            "pub fn post() {}\npub fn void() {}\n",
        )
        .expect("an edit");
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert!(
            found.broken.iter().any(|b| b.contains("names `void`")),
            "{found:?}"
        );
    }

    #[test]
    fn a_brace_set_in_touches_sees_a_change_to_either_file() {
        let root = repo("braces");
        let plan = PLAN.replace(
            "touches: [src/ledger.rs, src/refund.rs]",
            "touches: [\"src/{ledger,books}.rs\"]",
        );
        commit(
            &root,
            &[
                ("src/ledger.rs", "pub fn post() {}\n"),
                ("src/books.rs", "\n"),
                ("plan.md", plan.as_str()),
            ],
        );
        stamp(&root, "plan.md").expect("the plan is stamped");
        commit(&root, &[("src/books.rs", "pub fn record() {}\n")]);
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert!(
            found
                .drifted
                .iter()
                .any(|d| d.starts_with("src/books.rs changed since base")),
            "{found:?}"
        );
    }

    #[test]
    fn a_reference_into_history_is_read_from_git() {
        let root = repo("history");
        commit(&root, &[("src/old.rs", "pub fn old() {}\n")]);
        let first = Git { root: &root }.head().expect("a head");
        git(&root, &["rm", "-q", "src/old.rs"]);
        let plan = PLAN.replace(
            "ref: src/ledger.rs, symbol: post",
            &format!("ref: \"{first}:src/old.rs\""),
        );
        commit(
            &root,
            &[
                ("src/ledger.rs", "pub fn post() {}\n"),
                ("plan.md", plan.as_str()),
            ],
        );
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert!(found.broken.is_empty(), "{found:?}");

        let wrong = plan.replace(
            &format!("ref: \"{first}:src/old.rs\""),
            &format!("ref: \"{first}:src/old.rs\", symbol: young"),
        );
        commit(&root, &[("plan.md", wrong.as_str())]);
        let found = judge(&root, "plan.md").expect("the plan is judged");
        assert!(
            found.broken.iter().any(|b| b.contains("names `young`")),
            "{found:?}"
        );
    }
}
