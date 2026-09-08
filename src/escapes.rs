//! The escape log: what a loop has already tried, and one worked way out.
//!
//! The rule prose is static: every run of `sf check` renders the same bytes,
//! so a model that reads the fix, edits, stays red and reads again receives no
//! signal that its first idea was wrong. This module is the second half of
//! that output — the edits the same finding key has already survived, and one
//! minimal diff that cleared the same rule elsewhere, both derived from the
//! tool's own observations rather than from anything a model wrote.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// What a model reads when the report carries the log: the rule's `fix`, then
/// what it already tried, then a worked example. Byte-stable output is the
/// failure being fixed, so the rendering is report-side (see `src/report.rs`);
/// this module only decides *whether* the trail has anything to say.
pub const ATTEMPTS_BEFORE_ESCAPE: usize = 3;
const MAX_ATTEMPTS: usize = 5;
const MAX_STORED_ESCAPES: usize = 8;
/// Bisect passes per capture, and the hunk count above which the bisect is
/// skipped in favour of the file-level fallback. Both are ceilings on the
/// cost of one capture: one rule, in-process, once per transition.
const MAX_CAPTURE_PASSES: usize = 6;
const MAX_CAPTURE_HUNKS: usize = 16;
/// A snapshot of the red state carries the changed files' contents. Past
/// these caps the capture is skipped rather than truncated — a partial
/// snapshot would bisect against trees that never existed.
const MAX_SNAPSHOT_FILES: usize = 24;
const MAX_SNAPSHOT_CHARS: usize = 512_000;

/// `.software-factory/escapes/`: written under the factory directory but
/// deliberately outside the factory lock's scope — every failed attempt would
/// otherwise be an edit to a locked file, which trains nobody.
pub const DIR_NAME: &str = ".software-factory/escapes";

type Store = BTreeMap<String, Entry>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Entry {
    /// Digests of the tree diffs that left this key red. Small enough to list
    /// in a report, stable enough to dedupe on: the same edit twice is one
    /// attempt, not two.
    attempts: Vec<String>,
    /// The working-tree content of the changed files when the key was last
    /// seen red. This is what a later green run bisects against; `None` when
    /// the diff was too large to snapshot honestly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    snapshot: Option<BTreeMap<String, String>>,
    /// Minimal worked repairs, newest last.
    #[serde(default)]
    escapes: Vec<Escape>,
    /// How many times an escape in this entry has been surfaced without a
    /// green on its rule following. The decay that keeps the log a queue for
    /// the catalog rather than a second permanent channel.
    #[serde(default)]
    surfaced: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Escape {
    /// The hunks that flip the key, as (before, after) text pairs.
    hunks: Vec<(String, String)>,
    /// Whether this repair lands in the guarded configuration.
    weakening: bool,
}

/// The finding key is the stable identity — the ratchet depends on it, so a
/// key survives the edits that fail to clear it (`checks::complexity` keys by
/// `path:symbol`, `text_pattern` by content digest). That stability is what
/// makes a repeat the *same* problem, not a neighbouring one.
fn log_path(root: &Path, rule_id: &str) -> std::path::PathBuf {
    root.join(DIR_NAME).join(format!("{rule_id}.json"))
}

fn read(root: &Path, rule_id: &str) -> Result<Store> {
    let path = log_path(root, rule_id);
    if !path.exists() {
        return Ok(Store::new());
    }
    let body = std::fs::read_to_string(&path)?;
    serde_json::from_str(&body)
        .with_context(|| format!("{} is not a valid escape log", path.display()))
}

fn write(root: &Path, rule_id: &str, store: &Store) -> Result<()> {
    let path = log_path(root, rule_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(store)?))?;
    Ok(())
}

/// A red run's record: the tree diff left this key untouched.
///
/// `tree_diff` is the caller's diff summary — the digest of the working-tree
/// diff, not its text. `changed` is the red-time content of the files that
/// diff touched, the snapshot a later green run bisects against. Both come
/// from the tool's own observations; nothing here reads a model.
pub fn record_attempt(
    root: &Path,
    rule_id: &str,
    key: &str,
    tree_diff: &str,
    changed: Option<&BTreeMap<String, String>>,
) -> Result<Option<String>> {
    let mut store = read(root, rule_id)?;
    let entry = store.entry(key.to_string()).or_default();
    if entry
        .attempts
        .last()
        .is_some_and(|previous| previous == tree_diff)
    {
        return Ok(None);
    }
    entry.attempts.push(tree_diff.to_string());
    let count = entry.attempts.len();
    if count > MAX_ATTEMPTS {
        entry.attempts.drain(0..count - MAX_ATTEMPTS);
    }
    // The newest red state is the one a green transition is measured from.
    // A snapshot that exceeds the caps is recorded as absent, not truncated:
    // a partial snapshot would bisect against a tree that never existed.
    match changed {
        Some(files) if snapshot_fits(files) => entry.snapshot = Some(files.clone()),
        Some(_) => entry.snapshot = None,
        None => entry.snapshot = None,
    }
    let logged = entry.attempts.len();
    write(root, rule_id, &store)?;
    Ok(Some(format!("attempt {logged}")))
}

fn snapshot_fits(files: &BTreeMap<String, String>) -> bool {
    files.len() <= MAX_SNAPSHOT_FILES
        && files.values().map(|body| body.len()).sum::<usize>() <= MAX_SNAPSHOT_CHARS
}

/// How many distinct edits have failed against this key so far.
pub fn attempts(root: &Path, rule_id: &str, key: &str) -> usize {
    read(root, rule_id)
        .ok()
        .and_then(|store| store.get(key).map(|e| e.attempts.len()))
        .unwrap_or(0)
}

/// Every tracked key for this rule that still has a red-time snapshot. The
/// caller compares it with the current findings to identify red→green
/// transitions, because a cleared key is absent from the current report.
pub fn pending_keys(root: &Path, rule_id: &str) -> Result<Vec<String>> {
    Ok(read(root, rule_id)?
        .into_iter()
        .filter_map(|(key, entry)| {
            (!entry.attempts.is_empty() && entry.snapshot.is_some()).then_some(key)
        })
        .collect())
}

/// Rule buckets currently present in the local escape log.
pub fn pending_rules(root: &Path) -> Result<Vec<String>> {
    let directory = root.join(DIR_NAME);
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Ok(Vec::new());
    };
    let mut rules = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        rules.push(name.to_string());
    }
    rules.sort();
    rules.dedup();
    Ok(rules)
}

/// The worked escape to offer, ranked by proximity to the stuck key. `None`
/// while the trail is short: a model still on its first or second idea does
/// not need somebody else's answer, it needs to read the `fix` and vary.
pub fn retrieve(root: &Path, rule_id: &str, key: &str) -> Result<Option<String>> {
    let store = read(root, rule_id)?;
    let attempts = store.get(key).map(|e| e.attempts.len()).unwrap_or(0);
    if attempts < ATTEMPTS_BEFORE_ESCAPE {
        return Ok(None);
    }
    let same_file = key.split(':').next().unwrap_or("");
    let mut ranked: Vec<(&String, &Entry)> = store
        .iter()
        .filter(|(k, e)| k.as_str() != key && !e.escapes.is_empty())
        .collect();
    ranked.sort_by_key(|(k, _)| proximity(key, same_file, k));
    // Only the nearest entry's escapes are offered. An escape for a different
    // rule cannot be in this store at all, because one file per rule.
    let Some((_, entry)) = ranked.first() else {
        return Ok(None);
    };
    let escape = entry.escapes.last().expect("filtered for non-empty");
    if escape.weakening {
        return Ok(None);
    }
    Ok(Some(render(escape)))
}

fn render(escape: &Escape) -> String {
    let mut out = String::from("a worked repair for this rule, from elsewhere in this repository:");
    for (before, after) in &escape.hunks {
        out.push_str(&format!(
            "\n- before: {}\n  after:  {}",
            before.trim(),
            after.trim()
        ));
    }
    out
}

/// Distance between two finding keys: 1 same file, 2 same directory, 3
/// anything else in the rule's bucket. The stuck key itself is excluded by
/// the caller.
fn proximity(key: &str, same_file: &str, other: &str) -> usize {
    if other.split(':').next() == Some(same_file) {
        1
    } else if parent_of(key) == parent_of(other) {
        2
    } else {
        3
    }
}

fn parent_of(key: &str) -> String {
    key.split(':')
        .next()
        .unwrap_or(key)
        .rsplit_once('/')
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_default()
}

/// Record a green on this rule and decay what it made stale.
///
/// The keys this run actually cleared leave with their history: the fix prose
/// worked, or an escape did, and keeping either would be a second copy of the
/// catalog's job — the promotion the plan defers is the catalog's to make.
/// Every other entry's escape just failed to help again, so its surfaced
/// count rises here, and the count's ceiling is where eviction happens.
pub fn record_green(root: &Path, rule_id: &str, cleared: &[String]) -> Result<()> {
    let mut store = read(root, rule_id)?;
    if store.is_empty() {
        return Ok(());
    }
    for (key, entry) in store.iter_mut() {
        if cleared.iter().any(|c| c == key) {
            entry.attempts.clear();
            entry.snapshot = None;
            entry.surfaced = 0;
            continue;
        }
        if !entry.escapes.is_empty() {
            entry.surfaced += 1;
        }
    }
    store.retain(|_, entry| entry.surfaced <= MAX_STORED_ESCAPES);
    let dead: Vec<String> = store
        .iter()
        .filter(|(_, e)| e.attempts.is_empty() && e.escapes.is_empty())
        .map(|(k, _)| k.clone())
        .collect();
    for key in dead {
        store.remove(&key);
    }
    if store.is_empty() {
        let _ = std::fs::remove_file(log_path(root, rule_id));
        return Ok(());
    }
    write(root, rule_id, &store)
}

/// A key that just cleared loses its trail without touching anything else's.
/// A trail is about an in-progress attempt; once the finding is gone, its
/// history would render into the next report with no finding under it.
pub fn clear_trail(root: &Path, rule_id: &str, key: &str) -> Result<()> {
    let mut store = read(root, rule_id)?;
    let Some(entry) = store.get_mut(key) else {
        return Ok(());
    };
    entry.attempts.clear();
    entry.snapshot = None;
    entry.surfaced = 0;
    if entry.escapes.is_empty() {
        store.remove(key);
    }
    if store.is_empty() {
        let _ = std::fs::remove_file(log_path(root, rule_id));
        return Ok(());
    }
    write(root, rule_id, &store)
}

/// The red-time snapshot stored for one key, if any. The caller uses it to
/// bisect a red→green transition; the store owns the only copy.
pub fn snapshot_of(
    root: &Path,
    rule_id: &str,
    key: &str,
) -> Result<Option<BTreeMap<String, String>>> {
    Ok(read(root, rule_id)?
        .get(key)
        .and_then(|entry| entry.snapshot.clone()))
}

/// Capture on the red→green transition: which of the changed hunks flips
/// `key` from red to green.
///
/// `snapshot` is the red-time content of the changed files, `green` the
/// current content of the same paths, `probe` the one rule run over a scratch
/// tree, returning the finding keys it still reports. Cheap by construction:
/// one rule, in-process, a bounded number of bisect passes, and a fallback to
/// the whole-file before/after when the ceiling is hit.
pub fn capture_escape(
    root: &Path,
    rule_id: &str,
    key: &str,
    snapshot: &BTreeMap<String, String>,
    green: &BTreeMap<String, String>,
    probe: &dyn Fn(&Path) -> Result<Vec<String>>,
) -> Result<bool> {
    let hunks = hunks_between(snapshot, green);
    if hunks.is_empty() {
        return Ok(false);
    }
    // The bisect works on the file carrying the finding's location first; a
    // fix usually lands there, and never bisecting more than the ceiling is
    // the bound the plan promises.
    let file = file_of(key);
    let relevant: Vec<&Hunk> = hunks.iter().filter(|h| h.file == file).collect();
    let candidates: Vec<&Hunk> = if relevant.is_empty() {
        hunks.iter().collect()
    } else {
        relevant
    };
    if candidates.len() > MAX_CAPTURE_HUNKS {
        return store_fallback(root, rule_id, key, snapshot, green, &file);
    }
    let minimal = minimise(key, &candidates, probe)?;
    let Some(minimal) = minimal else {
        return store_fallback(root, rule_id, key, snapshot, green, &file);
    };
    store_escape(root, rule_id, key, minimal)
}

/// The whole-file before/after for the finding's own path. Coarse, but it is
/// the diff a human would have read, and it is what the plan's fallback
/// promises. Stored as a single escape, classified the same way.
fn store_fallback(
    root: &Path,
    rule_id: &str,
    key: &str,
    snapshot: &BTreeMap<String, String>,
    green: &BTreeMap<String, String>,
    file: &str,
) -> Result<bool> {
    let Some(before) = snapshot.get(file) else {
        return Ok(false);
    };
    let Some(after) = green.get(file) else {
        return Ok(false);
    };
    if before == after {
        return Ok(false);
    }
    store_escape(
        root,
        rule_id,
        key,
        Escape {
            hunks: vec![(format!("{file}\n{before}"), format!("{file}\n{after}"))],
            weakening: weakening(&[file.to_string()]),
        },
    )
}

fn store_escape(root: &Path, rule_id: &str, key: &str, escape: Escape) -> Result<bool> {
    let mut store = read(root, rule_id)?;
    let entry = store.entry(key.to_string()).or_default();
    entry.escapes.push(escape);
    if entry.escapes.len() > MAX_STORED_ESCAPES {
        entry
            .escapes
            .drain(0..entry.escapes.len() - MAX_STORED_ESCAPES);
    }
    write(root, rule_id, &store)?;
    Ok(true)
}

/// The guarded prefixes an escape must never be rendered from. A hunk touching
/// the policy, the ratchet, the rules, a lock or the allowlist is a weakening
/// — the highest-success-rate move an agent can make, which is exactly why it
/// is recorded for review rather than offered as advice.
const GUARDED_PREFIXES: &[&str] = &[
    ".software-factory/policy.yaml",
    ".software-factory/ratchet.yaml",
    ".software-factory/rules/",
    ".software-factory/catalog.lock.json",
    ".software-factory/locks/",
    ".allowed-root-files",
];

fn weakening(paths: &[String]) -> bool {
    paths.iter().any(|path| {
        GUARDED_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
    })
}

/// The minimal subset of hunks whose absence from the green tree brings the
/// finding back. Delta-debugging over the hunks: start with every hunk
/// removed and shrink while the key stays red.
fn minimise(
    key: &str,
    candidates: &[&Hunk],
    probe: &dyn Fn(&Path) -> Result<Vec<String>>,
) -> Result<Option<Escape>> {
    // One hunk: the minimal subset is that hunk, and one pass proves the
    // finding returns without it.
    if candidates.len() == 1 {
        let single: Vec<&Hunk> = vec![candidates[0]];
        if brings_back(key, &single, probe)? {
            return Ok(Some(escape_of(&single)));
        }
        return Ok(None);
    }
    // Binary search on the removal set: the largest tested prefix that keeps
    // the key red is the fix. Coarser than ddmin, but every step is one probe
    // and the ceiling keeps the worst case bounded.
    let mut low = 0usize;
    let mut high = candidates.len();
    let mut passes = 0usize;
    while low + 1 < high && passes < MAX_CAPTURE_PASSES {
        let mid = (low + high) / 2;
        if brings_back(key, &candidates[..mid], probe)? {
            high = mid;
        } else {
            low = mid;
        }
        passes += 1;
    }
    if passes >= MAX_CAPTURE_PASSES && low + 1 < high {
        return Ok(None);
    }
    if !brings_back(key, &candidates[..high], probe)? {
        return Ok(None);
    }
    Ok(Some(escape_of(&candidates[..high])))
}

fn brings_back(
    key: &str,
    removed: &[&Hunk],
    probe: &dyn Fn(&Path) -> Result<Vec<String>>,
) -> Result<bool> {
    let scratch = Scratch::new("bisect");
    let root = scratch.0.as_path();
    // Build the variant tree: green, with the removed hunks' old side put
    // back. Writing whole files keeps the variant self-consistent even when
    // several hunks share a file.
    for hunk in removed {
        let path = root.join(&hunk.file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut content = match std::fs::read_to_string(&path) {
            Ok(existing) => existing,
            Err(_) => hunk.after.clone(),
        };
        if !hunk.after.is_empty() {
            content = content.replace(&hunk.after, &hunk.before);
        } else {
            content = format!("{content}\n{before}", before = hunk.before);
        }
        std::fs::write(&path, content)?;
    }
    Ok(probe(root)?.iter().any(|found| found == key))
}

struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is before the epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("sf-escapes-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).expect("scratch directory");
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn escape_of(removed: &[&Hunk]) -> Escape {
    Escape {
        hunks: removed
            .iter()
            .map(|h| {
                (
                    format!("{} (line {})", h.file, h.old_start),
                    format!("{}\n{}", h.before.trim_end(), h.after.trim_end()),
                )
            })
            .collect(),
        weakening: weakening(&removed.iter().map(|h| h.file.clone()).collect::<Vec<_>>()),
    }
}

fn file_of(key: &str) -> String {
    key.split(':').next().unwrap_or(key).to_string()
}

struct Hunk {
    file: String,
    /// 1-based start line in the before file.
    old_start: usize,
    before: String,
    after: String,
}

/// The line-level diff between the red snapshot and the green tree, per file.
/// A common prefix/suffix trim yields one hunk for a single local edit, which
/// is the shape the bisect's fixtures exercise; several independent edits in
/// one file arrive as one hunk and the bisect still returns a correct, if
/// coarser, subset.
fn hunks_between(
    snapshot: &BTreeMap<String, String>,
    green: &BTreeMap<String, String>,
) -> Vec<Hunk> {
    let mut out = Vec::new();
    let mut files: Vec<&String> = snapshot.keys().chain(green.keys()).collect();
    files.sort();
    files.dedup();
    for file in files {
        let before = snapshot.get(file).map(String::as_str).unwrap_or("");
        let after = green.get(file).map(String::as_str).unwrap_or("");
        if before == after {
            continue;
        }
        let old_lines: Vec<&str> = before.lines().collect();
        let new_lines: Vec<&str> = after.lines().collect();
        let mut start = 0usize;
        while start < old_lines.len()
            && start < new_lines.len()
            && old_lines[start] == new_lines[start]
        {
            start += 1;
        }
        let mut old_end = old_lines.len();
        let mut new_end = new_lines.len();
        while old_end > start && new_end > start && old_lines[old_end - 1] == new_lines[new_end - 1]
        {
            old_end -= 1;
            new_end -= 1;
        }
        if old_end == start && new_end == start {
            continue;
        }
        out.push(Hunk {
            file: file.clone(),
            old_start: start + 1,
            before: old_lines[start..old_end].join("\n"),
            after: new_lines[start..new_end].join("\n"),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock is before the epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("sf-escapes-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("scratch directory");
            Scratch(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn snapshot() -> BTreeMap<String, String> {
        BTreeMap::from([("src/a.py".to_string(), "x = 1\n".to_string())])
    }

    #[test]
    fn the_same_key_in_two_runs_is_one_attempt() {
        let scratch = Scratch::new("attempt");
        let root = scratch.0.as_path();
        let first = record_attempt(
            root,
            "L1.COMPLEXITY_CEILING",
            "src/a.py:price",
            "diff-a",
            Some(&snapshot()),
        );
        assert_eq!(first.ok().flatten().as_deref(), Some("attempt 1"));
        let again = record_attempt(
            root,
            "L1.COMPLEXITY_CEILING",
            "src/a.py:price",
            "diff-a",
            None,
        );
        assert_eq!(
            again.ok().flatten().as_deref(),
            None,
            "a repeated diff summary is the same attempt"
        );
        assert_eq!(attempts(root, "L1.COMPLEXITY_CEILING", "src/a.py:price"), 1);
    }

    #[test]
    fn a_different_summary_is_a_second_attempt() {
        let scratch = Scratch::new("attempt-two");
        let root = scratch.0.as_path();
        let _ = record_attempt(root, "L1.C", "src/a.py:price", "diff x", Some(&snapshot()));
        let second = record_attempt(root, "L1.C", "src/a.py:price", "diff y", None);
        assert_eq!(second.ok().flatten().as_deref(), Some("attempt 2"));
        assert_eq!(attempts(root, "L1.C", "src/a.py:price"), 2);
    }

    #[test]
    fn the_trail_is_capped() {
        let scratch = Scratch::new("attempt-cap");
        let root = scratch.0.as_path();
        for i in 0..10 {
            record_attempt(root, "L1.C", "k", &format!("diff {i}"), None).expect("the call succeeds in this fixture");
        }
        assert_eq!(attempts(root, "L1.C", "k"), MAX_ATTEMPTS);
    }

    #[test]
    fn a_new_key_starts_a_fresh_trail() {
        let scratch = Scratch::new("attempt-new-key");
        let root = scratch.0.as_path();
        record_attempt(root, "L1.C", "src/a.py:price", "diff x", None).expect("the call succeeds in this fixture");
        assert_eq!(attempts(root, "L1.C", "src/a.py:parse"), 0);
    }

    #[test]
    fn the_trail_lives_outside_the_factory_locks_scope() {
        // The lock's scope is the policy, the ratchet, `.software-factory/rules/**`,
        // the allowlist, the workflows and the hooks. The escape log sits beside
        // them at `.software-factory/escapes/` and must not read as factory
        // configuration, or every failed attempt becomes an edit to a locked file.
        for path in [
            ".software-factory/policy.yaml",
            ".software-factory/ratchet.yaml",
            ".software-factory/rules/x.yaml",
            ".software-factory/locks/factory.lock.json",
            ".software-factory/catalog.lock.json",
        ] {
            assert!(
                !DIR_NAME.contains(path.trim_end_matches('/')),
                "{path} is factory configuration; {DIR_NAME} must sit outside every locked prefix"
            );
        }
        // And the concrete path shape: the lock globs name specific entries
        // under `.software-factory/`, none of which is `escapes`.
        assert_eq!(
            ".software-factory/rules/**"
                .matches(".software-factory/escapes/L1.C.json")
                .count(),
            0,
            "the rules glob must not reach the escapes directory"
        );
    }

    #[test]
    fn an_oversized_snapshot_is_recorded_as_absent_not_truncated() {
        let scratch = Scratch::new("snapshot-cap");
        let root = scratch.0.as_path();
        let huge: BTreeMap<String, String> =
            BTreeMap::from([("src/big.py".to_string(), "x".repeat(MAX_SNAPSHOT_CHARS + 1))]);
        record_attempt(root, "L1.C", "k", "diff", Some(&huge)).expect("the call succeeds in this fixture");
        let store = read(root, "L1.C").expect("the call succeeds in this fixture");
        assert!(
            store["k"].snapshot.is_none(),
            "a partial snapshot bisects a tree that never existed"
        );
    }
}

#[cfg(test)]
mod retrieval_and_decay {
    use super::*;

    fn escape_with(hunks: Vec<(String, String)>, weakening: bool) -> Escape {
        Escape { hunks, weakening }
    }

    #[test]
    fn one_store_per_rule() {
        assert_ne!(
            log_path(Path::new("/r"), "L1.COMPLEXITY_CEILING"),
            log_path(Path::new("/r"), "L1.COMPLEXITY_CEILING@legacy"),
            "even an instance keeps its own store, or an escape crosses between instances"
        );
    }

    #[test]
    fn nothing_is_offered_before_the_trail_is_long() {
        let scratch = Scratch::new("ladder");
        let root = scratch.0.as_path();
        record_attempt(root, "L1.C", "src/a.py:price", "diff", None).expect("the call succeeds in this fixture");
        assert!(retrieve(root, "L1.C", "src/a.py:price").expect("the call succeeds in this fixture").is_none());
    }

    #[test]
    fn a_weakening_is_never_rendered_as_advice() {
        let scratch = Scratch::new("weakening");
        let root = scratch.0.as_path();
        let mut store = Store::new();
        let mut stuck = Entry::default();
        for i in 0..ATTEMPTS_BEFORE_ESCAPE {
            stuck.attempts.push(format!("diff {i}"));
        }
        store.insert("src/a.py:price".to_string(), stuck);
        let mut solved = Entry::default();
        solved.escapes.push(escape_with(
            vec![(
                ".software-factory/ratchet.yaml".to_string(),
                "allow: []".to_string(),
            )],
            true,
        ));
        store.insert("src/b.py:total".to_string(), solved);
        write(root, "L1.C", &store).expect("the store writes");

        assert!(
            retrieve(root, "L1.C", "src/a.py:price").expect("the call succeeds in this fixture").is_none(),
            "a weakening must never be offered as advice"
        );
    }

    #[test]
    fn ranking_prefers_the_same_file_then_the_same_directory() {
        let mut ranked: Vec<(&str, usize)> = [
            ("src/orders/total.py:total", 0),
            ("docs/guide.md:section", 0),
            ("src/pricing.py:rate", 0),
        ]
        .map(|(k, _)| {
            let file = k.split(':').next().unwrap_or(k);
            (
                k,
                proximity("src/orders/service.py:price", "src/orders/service.py", file),
            )
        })
        .to_vec();
        ranked.sort_by_key(|(_, distance)| *distance);
        assert_eq!(
            ranked[0].0, "src/orders/total.py:total",
            "same directory outranks everything else"
        );
        // The plan's ladder is key, file, directory, then "any escape for the
        // rule" — the last bucket carries no finer order, so both remaining
        // candidates rank equal at 3.
        assert_eq!(
            ranked[1].1, 3,
            "outside the directory is the catch-all rank"
        );
        assert_eq!(ranked[2].1, 3, "docs and src tree are the same distance");
    }

    #[test]
    fn an_entry_never_followed_by_a_green_is_evicted() {
        let scratch = Scratch::new("evict");
        let root = scratch.0.as_path();
        let mut store = Store::new();
        let mut stuck = Entry::default();
        stuck.attempts.push("diff".to_string());
        store.insert("src/a.py:price".to_string(), stuck);
        let mut solved = Entry::default();
        solved
            .escapes
            .push(escape_with(vec![("a".to_string(), "b".to_string())], false));
        store.insert("src/b.py:total".to_string(), solved);
        write(root, "L1.C", &store).expect("the store writes");

        // Greens on other keys, never on the escape's own: it keeps being
        // surfaced and keeps not helping.
        for _ in 0..=MAX_STORED_ESCAPES {
            record_green(root, "L1.C", &["src/a.py:price".to_string()]).expect("the call succeeds in this fixture");
        }
        let store = read(root, "L1.C").expect("the call succeeds in this fixture");
        assert!(
            !store
                .get("src/b.py:total")
                .map(|e| !e.escapes.is_empty())
                .unwrap_or(false),
            "an escape that never precedes a green is evicted"
        );
    }

    /// The healthy end of an escape: the rule's own `fix` prose absorbed it.
    /// Keeping a cleared key's history would be a second permanent channel
    /// competing with the catalog.
    #[test]
    fn a_green_on_the_key_clears_its_history() {
        let scratch = Scratch::new("promote");
        let root = scratch.0.as_path();
        let mut store = Store::new();
        let mut stuck = Entry::default();
        stuck.attempts.push("diff".to_string());
        store.insert("src/a.py:price".to_string(), stuck);
        write(root, "L1.C", &store).expect("the store writes");
        record_green(root, "L1.C", &["src/a.py:price".to_string()]).expect("the call succeeds in this fixture");
        assert!(
            read(root, "L1.C").expect("the store reads").is_empty(),
            "the key's trail and escapes leave with it"
        );
        assert!(
            !log_path(root, "L1.C").exists(),
            "an empty store is not a file nobody reads"
        );
    }
}

#[cfg(test)]
mod hunks {
    use super::*;

    #[test]
    fn one_local_edit_is_one_hunk() {
        let snapshot = BTreeMap::from([(
            "src/pricing.py".to_string(),
            "def price(order):\n    total = 0\n    return total\n".to_string(),
        )]);
        let green = BTreeMap::from([(
            "src/pricing.py".to_string(),
            "def price(order):\n    total = 1\n    return total\n".to_string(),
        )]);
        let hunks = hunks_between(&snapshot, &green);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].file, "src/pricing.py");
        assert!(hunks[0].before.contains("total = 0"));
        assert!(hunks[0].after.contains("total = 1"));
        assert_eq!(hunks[0].old_start, 2);
    }

    #[test]
    fn identical_content_produces_no_hunks() {
        let same = BTreeMap::from([("src/a.py".to_string(), "x\n".to_string())]);
        assert!(hunks_between(&same, &same).is_empty());
    }

    /// The classification is the part that keeps the log trustworthy, so it
    /// is pinned against the real prefix list rather than a test-local copy.
    #[test]
    fn every_guarded_prefix_is_factory_configuration() {
        for prefix in GUARDED_PREFIXES {
            assert!(
                prefix.starts_with(".software-factory/") || *prefix == ".allowed-root-files",
                "{prefix} guards something outside the factory's own files?"
            );
        }
        for expected in [
            ".software-factory/policy.yaml",
            ".software-factory/ratchet.yaml",
            ".software-factory/rules/",
            ".software-factory/locks/",
            ".software-factory/catalog.lock.json",
        ] {
            assert!(
                GUARDED_PREFIXES.contains(&expected),
                "the classifier stopped watching {expected}"
            );
        }
    }

    #[test]
    fn a_hunk_touching_the_ratchet_is_classified_a_weakening() {
        assert!(weakening(&[".software-factory/ratchet.yaml".to_string()]));
        assert!(!weakening(&["src/pricing.py".to_string()]));
        // A path that merely contains a guarded name is not under it.
        assert!(!weakening(&[
            "src/.software-factory/ratchet.yaml.bak".to_string()
        ]));
    }
}

#[cfg(test)]
mod capture {
    use super::*;
    use crate::catalog::Catalog;
    use crate::checks;
    use crate::policy::Policy;
    use crate::ratchet::Ratchet;
    use crate::scan;
    use std::path::PathBuf;

    const POLICY: &str = "version: 1\n\
         project:\n  name: capture\n  languages: [python]\n\
         rules:\n  L1.COMPLEXITY_CEILING:\n    enabled: true\n    options:\n      max: 4\n";

    /// The in-process probe the bisect runs: seed the scratch tree with the
    /// minimal factory policy, run the one rule, return the finding keys.
    /// Same shape as the real probe `cmd_check` hands in — the bisect scratch
    /// is a repository as far as the rule is concerned — without a subprocess.
    fn probe_factory() -> impl Fn(&Path) -> Result<Vec<String>> {
        |root: &Path| {
            std::fs::create_dir_all(root.join(".software-factory"))?;
            std::fs::write(root.join(".software-factory/policy.yaml"), POLICY)?;
            let policy = Policy::load(root)?;
            let catalog = Catalog::builtin()?;
            let files = scan::walk(root, &policy)?;
            let ratchet = Ratchet::default();
            let ctx = checks::Ctx {
                root,
                policy: &policy,
                catalog: &catalog,
                files: &files,
                ratchet: &ratchet,
                changed: None,
                base: None,
                today: crate::clock::today(),
                allow_commands: false,
            };
            let rule = catalog.get("L1.COMPLEXITY_CEILING").expect("ships");
            Ok(checks::run_one(rule, &ctx)?
                .into_iter()
                .map(|f| f.key)
                .collect())
        }
    }

    fn scratch_repo(tag: &str, content: &str) -> (Scratch, PathBuf) {
        let scratch = Scratch::new(tag);
        let root = scratch.0.clone();
        std::fs::create_dir_all(scratch.0.join(".software-factory")).expect("factory dir");
        std::fs::create_dir_all(scratch.0.join("src")).expect("src dir");
        std::fs::write(scratch.0.join(".software-factory/policy.yaml"), POLICY).expect("policy");
        std::fs::write(scratch.0.join("src/pricing.py"), content).expect("source");
        (scratch, root)
    }

    fn red() -> String {
        "def price(order):\n    total = 0\n    for a in order.x:\n        total += 1\n    for b in order.y:\n        total += 1\n    for c in order.z:\n        total += 1\n    for d in order.w:\n        total += 1\n    return total\n"
            .to_string()
    }

    fn green() -> String {
        "def price(order):\n    total = 0\n    return total\n".to_string()
    }

    fn with_noise(green: &str) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("src/pricing.py".to_string(), green.to_string()),
            // The co-changed noise a real diff is mostly made of, in its own
            // file: per-file hunks are what let the bisect drop it.
            ("src/notes.py".to_string(), "NOISE = 1\n".to_string()),
        ])
    }

    fn keys_of(root: &Path, content: &str) -> Vec<String> {
        std::fs::write(root.join("src/pricing.py"), content).expect("probe content");
        probe_factory()(root).expect("the probe runs")
    }

    /// One fix hunk plus one noise hunk: the minimal set whose absence brings
    /// the finding back is the fix, and the noise never joins it.
    #[test]
    fn the_bisect_finds_the_hunk_that_flips_the_key() {
        let (scratch, root) = scratch_repo("bisect", &red());
        // Confirm the starting state really is red on this key.
        assert!(
            keys_of(&root, &red()).iter().any(|k| k.contains("price")),
            "the red side trips the rule"
        );
        let snapshot = BTreeMap::from([("src/pricing.py".to_string(), red())]);
        let tree = with_noise(&green());
        let captured = capture_escape(
            &root,
            "L1.COMPLEXITY_CEILING",
            "src/pricing.py:price",
            &snapshot,
            &tree,
            &probe_factory(),
        )
        .expect("the capture runs");
        assert!(captured, "a transition with a fix hunk is captured");
        let store = read(&root, "L1.COMPLEXITY_CEILING").expect("the call succeeds in this fixture");
        let escape = &store["src/pricing.py:price"].escapes[0];
        assert!(!escape.weakening, "a source-file repair is not a weakening");
        let rendered = render(escape);
        assert!(
            rendered.contains("for a in order.x"),
            "the fix hunks carry the repair: {rendered}"
        );
        assert!(
            !rendered.contains("NOISE"),
            "the co-changed noise is not part of the minimal repair: {rendered}"
        );
        let _ = scratch;
    }

    #[test]
    fn removing_the_stored_hunks_reproduces_the_finding() {
        let (scratch, root) = scratch_repo("bisect-repro", &green());
        let snapshot = BTreeMap::from([("src/pricing.py".to_string(), red())]);
        let tree = BTreeMap::from([("src/pricing.py".to_string(), green())]);
        capture_escape(
            &root,
            "L1.COMPLEXITY_CEILING",
            "src/pricing.py:price",
            &snapshot,
            &tree,
            &probe_factory(),
        )
        .expect("the capture runs");
        let store = read(&root, "L1.COMPLEXITY_CEILING").expect("the call succeeds in this fixture");
        let escape = &store["src/pricing.py:price"].escapes[0];
        // The before side of the stored repair, applied to the green tree,
        // must bring the finding back. This is the property that makes the
        // stored diff a worked repair rather than a coincidence.
        let before = escape.hunks[0].0.clone();
        // The stored before is the finding's file plus the block the fix
        // removed; splicing it over the green file rebuilds the red function.
        let body = before
            .split_once('\n')
            .map(|(_, rest)| rest)
            .unwrap_or("")
            .to_string();
        let restored = body.to_string();
        let returned = keys_of(&root, &restored);
        assert!(
            returned.iter().any(|k| k.contains("src/pricing.py")),
            "the stored before-side reproduces the finding: {before}"
        );
        let _ = scratch;
    }

    #[test]
    fn too_many_hunks_falls_back_to_the_whole_file() {
        let (scratch, root) = scratch_repo("bisect-fallback", &green());
        // Many hunks across many files: above the ceiling, the fallback
        // stores the finding's own file whole.
        let mut snapshot = BTreeMap::new();
        let mut tree = BTreeMap::new();
        for i in 0..20 {
            let path = format!("src/generated{i}.py");
            snapshot.insert(path.clone(), format!("A = {i}\n"));
            tree.insert(path, format!("B = {i}\n"));
        }
        snapshot.insert("src/pricing.py".to_string(), red());
        tree.insert("src/pricing.py".to_string(), green());
        let captured = capture_escape(
            &root,
            "L1.COMPLEXITY_CEILING",
            "src/pricing.py:price",
            &snapshot,
            &tree,
            &probe_factory(),
        )
        .expect("the capture runs");
        assert!(captured, "the fallback still records a worked example");
        let store = read(&root, "L1.COMPLEXITY_CEILING").expect("the call succeeds in this fixture");
        let escape = &store["src/pricing.py:price"].escapes[0];
        assert!(
            escape.hunks[0].0.starts_with("src/pricing.py"),
            "the fallback is the file diff, not a bisected subset: {:?}",
            escape.hunks[0].0
        );
        let _ = scratch;
    }

    #[test]
    fn a_transition_the_files_do_not_explain_captures_nothing() {
        let (scratch, root) = scratch_repo("bisect-noop", &red());
        let snapshot = BTreeMap::from([("src/pricing.py".to_string(), red())]);
        // Green content identical to red: no hunk explains the transition.
        let tree = BTreeMap::from([("src/pricing.py".to_string(), red())]);
        let captured = capture_escape(
            &root,
            "L1.COMPLEXITY_CEILING",
            "src/pricing.py:price",
            &snapshot,
            &tree,
            &probe_factory(),
        )
        .expect("the capture runs");
        assert!(!captured, "nothing to bisect when the content did not move");
        let _ = scratch;
    }

    /// The capture only runs when a snapshot exists to bisect against: a
    /// trail that grew while the diff was too large never stores an escape.
    #[test]
    fn no_snapshot_means_no_escape() {
        let scratch = Scratch::new("no-snapshot");
        let root = scratch.0.as_path();
        record_attempt(root, "L1.C", "k", "diff", None).expect("the call succeeds in this fixture");
        assert!(
            pending_keys(root, "L1.C")
                .expect("the call succeeds in this fixture")
                .is_empty(),
            "no snapshot, no capture"
        );
    }
}
