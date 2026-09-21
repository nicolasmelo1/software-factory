//! This repository is public and some of the work that drove it is not. This
//! is the check that keeps the two apart.
//!
//! It is an integration test rather than a rule in the catalog, and rather
//! than a module under `src/`, because it is about *this* repository and not
//! about the tool. Nothing here ships in the `sf` binary; `cargo test` is
//! already what the pre-commit hook and CI run, so the check costs no new
//! wiring and cannot be forgotten.
//!
//! The forbidden terms are carried as digests rather than as words, for two
//! reasons that are really one. A list of a company's private names, written
//! out in a public repository, is the leak it exists to prevent; and a check
//! that spells out what it forbids reports itself on every run. So
//! `tests/private-terms.json` carries `sha256(term)` with the term's length,
//! and the text is reduced to the same shape before it is compared.
//!
//! Matching is by window rather than by word, so a term glued into a longer
//! identifier — `AcmeDeps`, `workflow-acme`, `ACME-7716` — is found the same
//! way a term standing on its own is. That is what makes the length part of
//! the policy: without it there is no window to hash. The examples here are
//! fictional on purpose: this file is read by the check it implements.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct Policy {
    denied: Vec<DeniedTerm>,
}

#[derive(Deserialize)]
struct DeniedTerm {
    /// Length of the term, which is the width of the window to hash.
    length: usize,
    /// Lowercase hex sha256 of the lowercased term.
    sha256: String,
}

#[derive(Debug, PartialEq, Eq)]
struct PrivateRef {
    file: String,
    /// 1-indexed, so the report is something an editor can be pointed at.
    line: usize,
    /// The word the term was found inside, so a person knows what to remove.
    word: String,
}

fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Every place a denied term appears, in reading order. An empty policy finds
/// nothing and says nothing — refusing it is the caller's job, because only
/// the caller knows whether an empty list means "nothing to hide" or "the file
/// did not load".
fn find_private_references(files: &[(String, String)], denied: &[DeniedTerm]) -> Vec<PrivateRef> {
    let mut found = Vec::new();
    let mut widths: Vec<usize> = denied.iter().map(|term| term.length).collect();
    widths.sort_unstable();
    widths.dedup();
    let digests: HashSet<&str> = denied.iter().map(|term| term.sha256.as_str()).collect();
    let Some(&narrowest) = widths.first() else {
        return found;
    };

    // A repository repeats its vocabulary constantly, and hashing is the only
    // expensive thing here. One answer per distinct word is the difference
    // between a check in the pre-commit hook and a check nobody runs.
    let mut judged: HashMap<String, bool> = HashMap::new();

    for (path, text) in files {
        for (index, line) in text.lines().enumerate() {
            for word in line
                .to_lowercase()
                .split(|ch: char| !ch.is_ascii_alphanumeric())
                .filter(|word| word.len() >= narrowest)
            {
                let carries = *judged.entry(word.to_owned()).or_insert_with(|| {
                    widths.iter().any(|&width| {
                        word.as_bytes()
                            .windows(width)
                            .filter_map(|window| std::str::from_utf8(window).ok())
                            .any(|window| digests.contains(digest(window).as_str()))
                    })
                });

                if carries {
                    found.push(PrivateRef {
                        file: path.clone(),
                        line: index + 1,
                        word: word.to_owned(),
                    });
                }
            }
        }
    }

    found
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The tracked text of the repository, which is exactly what is published.
fn tracked_text(root: &Path) -> Vec<(String, String)> {
    let listing = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .expect("git ls-files runs in a checkout");
    assert!(listing.status.success(), "git ls-files failed");

    String::from_utf8_lossy(&listing.stdout)
        .split('\0')
        .filter(|path| !path.is_empty())
        .filter_map(|path| {
            let absolute = root.join(path);
            let bytes = std::fs::read(&absolute).ok()?;
            // A NUL byte is the cheap, portable "this is not text". Hashing
            // windows of a binary would be slow and would report offsets
            // nobody can act on.
            if bytes.contains(&0) {
                return None;
            }
            Some((path.to_owned(), String::from_utf8(bytes).ok()?))
        })
        .collect()
}

fn policy(root: &Path) -> Policy {
    let raw = std::fs::read_to_string(root.join("tests/private-terms.json"))
        .expect("tests/private-terms.json is part of the gate and must be readable");
    serde_json::from_str(&raw).expect("tests/private-terms.json is valid JSON")
}

#[test]
fn no_private_name_reaches_this_public_repository() {
    let root = repo();
    let policy = policy(&root);

    // An empty list would pass every file in the repository while looking like
    // a check. Deleting the terms has to be as loud as tripping over one.
    assert!(
        !policy.denied.is_empty(),
        "tests/private-terms.json denies nothing, so this check proves nothing"
    );

    let files = tracked_text(&root);
    assert!(!files.is_empty(), "no tracked text found — is this a checkout?");

    let found = find_private_references(&files, &policy.denied);
    let report: Vec<String> = found
        .iter()
        .map(|hit| format!("  {}:{}  {}", hit.file, hit.line, hit.word))
        .collect();

    assert!(
        found.is_empty(),
        "a private name reached a public repository:\n{}\n\nRemove it. If the word is innocent \
         and the match is a coincidence, the term itself is too broad — narrow it rather than \
         adding an exception here.",
        report.join("\n")
    );
}

// The terms this repository actually forbids are never written below. A test
// that spells them out puts them back in the tree the check exists to keep
// them out of — and would be reported by the check on its next run. The rule
// is general, so a term nobody is hiding proves it just as well.
#[cfg(test)]
mod tests {
    use super::*;

    fn deny(term: &str) -> DeniedTerm {
        DeniedTerm {
            length: term.len(),
            sha256: digest(term),
        }
    }

    fn file(path: &str, text: &str) -> Vec<(String, String)> {
        vec![(path.to_owned(), text.to_owned())]
    }

    #[test]
    fn passes_a_tree_that_names_nothing_forbidden() {
        let files = file("README.md", "a policy engine that reads a repository");
        assert_eq!(find_private_references(&files, &[deny("acme")]), vec![]);
    }

    #[test]
    fn finds_a_term_standing_on_its_own_at_its_line() {
        let files = file("docs/note.md", "first line\nwritten for Acme after a defect");
        assert_eq!(
            find_private_references(&files, &[deny("acme")]),
            vec![PrivateRef {
                file: "docs/note.md".into(),
                line: 2,
                word: "acme".into()
            }]
        );
    }

    #[test]
    fn finds_a_term_glued_into_a_longer_identifier() {
        let files = file("src/deps.rs", "struct AcmeDeps {}");
        assert_eq!(
            find_private_references(&files, &[deny("acme")]),
            vec![PrivateRef {
                file: "src/deps.rs".into(),
                line: 1,
                word: "acmedeps".into()
            }]
        );
    }

    #[test]
    fn finds_a_term_a_separator_broke_apart() {
        let files = file("plans/a.md", "`ACME-7716` reached a real ticket");
        assert_eq!(
            find_private_references(&files, &[deny("acme")]),
            vec![PrivateRef {
                file: "plans/a.md".into(),
                line: 1,
                word: "acme".into()
            }]
        );
    }

    #[test]
    fn finds_a_term_inside_a_longer_private_name() {
        let files = file("Cargo.toml", "name = \"acmecorp-backend\"");
        assert_eq!(
            find_private_references(&files, &[deny("acme")]),
            vec![PrivateRef {
                file: "Cargo.toml".into(),
                line: 1,
                word: "acmecorp".into()
            }]
        );
    }

    #[test]
    fn reports_every_occurrence_so_one_fix_does_not_hide_the_next() {
        let files = vec![
            ("a.md".to_owned(), "acme\nnothing\nacme again".to_owned()),
            ("b.md".to_owned(), "acme".to_owned()),
        ];
        assert_eq!(
            find_private_references(&files, &[deny("acme")]),
            vec![
                PrivateRef { file: "a.md".into(), line: 1, word: "acme".into() },
                PrivateRef { file: "a.md".into(), line: 3, word: "acme".into() },
                PrivateRef { file: "b.md".into(), line: 1, word: "acme".into() },
            ]
        );
    }

    #[test]
    fn judges_terms_of_several_lengths_against_the_same_word() {
        let files = file("a.md", "the northwind board");
        assert_eq!(
            find_private_references(&files, &[deny("acme"), deny("northwind")]),
            vec![PrivateRef {
                file: "a.md".into(),
                line: 1,
                word: "northwind".into()
            }]
        );
    }

    #[test]
    fn is_blind_to_case_because_a_leak_does_not_care_how_it_was_typed() {
        let files = file("a.md", "AcMe");
        assert_eq!(
            find_private_references(&files, &[deny("acme")]),
            vec![PrivateRef {
                file: "a.md".into(),
                line: 1,
                word: "acme".into()
            }]
        );
    }

    #[test]
    fn does_not_report_a_word_that_merely_shares_letters_with_a_term() {
        let files = file("a.md", "came acre mace");
        assert_eq!(find_private_references(&files, &[deny("acme")]), vec![]);
    }

    #[test]
    fn finds_nothing_when_no_term_is_denied() {
        let files = file("a.md", "acme");
        assert_eq!(find_private_references(&files, &[]), vec![]);
    }
}
