//! What a package manifest declares about one dependency.
//!
//! A rule instance may say it is only about `tailwindcss ^3` — see `When` in
//! [`crate::policy`]. Deciding that costs exactly one fact from the
//! repository: the range its manifest declares for that package.
//!
//! The manifest range, never the resolved lock version. The lock is more
//! accurate and there are several lock formats per ecosystem, each with its
//! own schema and its own churn; the range in the manifest is what the team
//! decided, and the decision is what the rule is about.

use anyhow::Result;
use regex::Regex;
use std::path::Path;

/// The manifests this binary can read, for a message that has to name them.
pub const READABLE: &str =
    "package.json, Cargo.toml, pyproject.toml, requirements*.txt, Gemfile, go.mod";

/// What a manifest had to say about one dependency.
///
/// Every arm other than `Range` is a reason a `when` cannot be decided, and
/// each is reported rather than treated as "no", because a condition that
/// quietly answers no is a way to switch a rule off by editing a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declared {
    /// The range as written: `^3.4.1`, `>=0.110,<0.111`, `~> 7.1`.
    Range(String),
    /// The manifest was read and declares no such dependency.
    Absent,
    /// Nothing at that path.
    NoManifest,
    /// A file this binary has no reader for.
    UnknownFormat,
    /// A format this binary reads, which did not parse.
    Malformed(String),
}

/// The range `manifest` declares for `dependency`, read from disk.
pub fn declared(root: &Path, manifest: &str, dependency: &str) -> Result<Declared> {
    let Some(name) = Path::new(manifest).file_name().and_then(|n| n.to_str()) else {
        return Ok(Declared::UnknownFormat);
    };
    let Ok(body) = std::fs::read_to_string(root.join(manifest)) else {
        return Ok(Declared::NoManifest);
    };
    read(name, &body, dependency)
}

/// Dispatch on the manifest's file name. A name with no reader is
/// `UnknownFormat`: this tool declines to guess at an ecosystem it cannot
/// parse, and says so, rather than reporting the dependency absent.
fn read(name: &str, body: &str, dependency: &str) -> Result<Declared> {
    match name {
        "package.json" => Ok(from_package_json(body, dependency)),
        "Cargo.toml" => from_toml_tables(body, dependency),
        "pyproject.toml" => from_pyproject(body, dependency),
        "Gemfile" => from_gemfile(body, dependency),
        "go.mod" => from_go_mod(body, dependency),
        n if n.starts_with("requirements") && n.ends_with(".txt") => {
            Ok(from_requirements(body, dependency))
        }
        _ => Ok(Declared::UnknownFormat),
    }
}

fn from_package_json(body: &str, dependency: &str) -> Declared {
    let value: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => return Declared::Malformed(error.to_string()),
    };
    let sections =
        ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"];
    for section in sections {
        let range = value.get(section).and_then(|s| s.get(dependency)).and_then(|r| r.as_str());
        if let Some(range) = range {
            return Declared::Range(range.to_string());
        }
    }
    Declared::Absent
}

/// A TOML dependency table, in either form Cargo and Poetry accept:
/// `serde = "1.0"`, `serde = { version = "1.0", features = [...] }`, and the
/// separate-table `[dependencies.serde]` with a `version` key under it.
fn from_toml_tables(body: &str, dependency: &str) -> Result<Declared> {
    let entry = Regex::new(&format!(
        r#"^\s*"?{}"?\s*=\s*(.+?)\s*$"#,
        regex::escape(dependency)
    ))?;
    let version = Regex::new(r#"^\s*version\s*=\s*(.+?)\s*$"#)?;
    let own_table = format!("dependencies.{dependency}");
    let mut section = String::new();
    for line in body.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
            section = header.trim().to_string();
            continue;
        }
        if section.ends_with(&own_table)
            && let Some(found) = version.captures(line)
        {
            return Ok(Declared::Range(toml_value(&found[1])));
        }
        if section.ends_with("dependencies")
            && let Some(found) = entry.captures(line)
        {
            return Ok(Declared::Range(toml_value(&found[1])));
        }
    }
    Ok(Declared::Absent)
}

/// The version out of a TOML right-hand side. A string is itself; an inline
/// table is its `version` key; anything else is handed back verbatim so the
/// caller can say what it could not read rather than inventing a range.
fn toml_value(raw: &str) -> String {
    if let Some(inner) = raw.strip_prefix('"').and_then(|r| r.split('"').next()) {
        return inner.to_string();
    }
    if raw.starts_with('{')
        && let Some(found) = Regex::new(r#"version\s*=\s*"([^"]*)""#).ok().and_then(|r| {
            r.captures(raw).map(|c| c[1].to_string())
        })
    {
        return found;
    }
    raw.to_string()
}

/// PEP 621 first (`dependencies = ["fastapi>=0.110"]`, and the optional and
/// group tables that share its shape), then Poetry's dependency tables.
///
/// A line that assigns a bare string to a key — `name = "fastapi"` — is left
/// to the table pass. Otherwise a project that shares a name with something it
/// depends on answers the wrong question here.
fn from_pyproject(body: &str, dependency: &str) -> Result<Declared> {
    let quoted = Regex::new(r#"["']([^"']+)["']"#)?;
    let scalar = Regex::new(r#"^\s*[A-Za-z0-9_.\-"']+\s*=\s*["']"#)?;
    for line in body.lines() {
        if scalar.is_match(line) {
            continue;
        }
        for capture in quoted.captures_iter(line) {
            if let Some((name, range)) = requirement(&capture[1])
                && same_package(&name, dependency)
            {
                return Ok(Declared::Range(range));
            }
        }
    }
    from_toml_tables(body, dependency)
}

fn from_requirements(body: &str, dependency: &str) -> Declared {
    for line in body.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('-') {
            continue;
        }
        if let Some((name, range)) = requirement(line)
            && same_package(&name, dependency)
        {
            return Declared::Range(range);
        }
    }
    Declared::Absent
}

/// `fastapi[all]>=0.110,<0.111` → (`fastapi`, `>=0.110,<0.111`).
fn requirement(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
        .unwrap_or(text.len());
    if end == 0 {
        return None;
    }
    let (name, rest) = text.split_at(end);
    let rest = rest.trim_start();
    let rest = match rest.strip_prefix('[') {
        Some(extras) => extras.split_once(']').map(|(_, after)| after).unwrap_or(""),
        None => rest,
    };
    Some((name.to_string(), rest.trim().to_string()))
}

/// PEP 503 name equivalence: `Flask_SQLAlchemy` and `flask-sqlalchemy` are one
/// package, and a `when` must not turn on how somebody typed it.
fn same_package(a: &str, b: &str) -> bool {
    let normalize = |name: &str| {
        name.to_ascii_lowercase().replace(['_', '.'], "-")
    };
    normalize(a) == normalize(b)
}

/// `gem "rails", "~> 7.1"`. The quoted arguments that are constraints are the
/// ones starting with a digit or a comparison operator — the rest are the
/// `git:` and `path:` options, which name no version.
fn from_gemfile(body: &str, dependency: &str) -> Result<Declared> {
    let line = Regex::new(&format!(
        r#"(?m)^\s*gem\s+["']{}["']\s*(.*)$"#,
        regex::escape(dependency)
    ))?;
    let Some(found) = line.captures(body) else {
        return Ok(Declared::Absent);
    };
    let arguments = found[1].split('#').next().unwrap_or("");
    let quoted = Regex::new(r#"["']([^"']+)["']"#)?;
    let constraints: Vec<String> = quoted
        .captures_iter(arguments)
        .map(|c| c[1].trim().to_string())
        .filter(|value| {
            value.starts_with(|c: char| c.is_ascii_digit()) || value.starts_with(['~', '>', '<', '='])
        })
        .collect();
    Ok(Declared::Range(constraints.join(", ")))
}

/// `require github.com/spf13/cobra v1.8.0`, inside a block or on its own line.
fn from_go_mod(body: &str, dependency: &str) -> Result<Declared> {
    let line = Regex::new(&format!(
        r"(?m)^\s*(?:require\s+)?{}\s+(\S+)",
        regex::escape(dependency)
    ))?;
    Ok(match line.captures(body) {
        Some(found) => Declared::Range(found[1].to_string()),
        None => Declared::Absent,
    })
}

/// A dotted release number, compared component by component.
///
/// Deliberately not a semver implementation: pre-release tags, build metadata
/// and the ordering rules that go with them are not what a `when` is deciding.
/// A `when` asks whether the pin is still in the series the rule was written
/// for, and that question is answered by the numbers.
#[derive(Debug, Clone, Eq)]
pub struct Version(pub Vec<u64>);

/// Equality is the ordering's, not the vector's: a version that stops short
/// is the same release as one that spells its zeroes out, and a derived
/// `PartialEq` would disagree with `Ord` about exactly that.
impl PartialEq for Version {
    fn eq(&self, other: &Version) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Version {
    /// The first release number in a piece of text: `^3.4.1` → 3.4.1,
    /// `>=0.110,<0.111` → 0.110, `v1.8.0` → 1.8.0. `None` when the text
    /// carries no number at all (`*`, `latest`, a git URL), which is a
    /// manifest entry this tool refuses to guess at.
    pub fn parse(text: &str) -> Option<Version> {
        let start = text.find(|c: char| c.is_ascii_digit())?;
        let rest = &text[start..];
        let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
        let parts: Vec<u64> = rest[..end]
            .split('.')
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse().ok())
            .collect();
        if parts.is_empty() {
            return None;
        }
        Some(Version(parts))
    }

    /// The component at `index`, where a version that stops short reads as
    /// zero: `3` and `3.0.0` are the same release.
    pub fn at(&self, index: usize) -> u64 {
        self.0.get(index).copied().unwrap_or(0)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> std::cmp::Ordering {
        let width = self.0.len().max(other.0.len());
        (0..width)
            .map(|index| self.at(index).cmp(&other.at(index)))
            .find(|ordering| ordering.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[cfg(test)]
mod reading {
    use super::*;

    #[test]
    fn every_readable_manifest_yields_the_declared_range() {
        let cases: &[(&str, &str, &str, &str)] = &[
            (
                "package.json",
                r#"{"dependencies": {"tailwindcss": "^3.4.1"}}"#,
                "tailwindcss",
                "^3.4.1",
            ),
            (
                "package.json",
                r#"{"devDependencies": {"vitest": "~1.2.0"}}"#,
                "vitest",
                "~1.2.0",
            ),
            ("Cargo.toml", "[dependencies]\nserde = \"1.0.229\"\n", "serde", "1.0.229"),
            (
                "Cargo.toml",
                "[dependencies]\nclap = { version = \"4.6.6\", features = [\"derive\"] }\n",
                "clap",
                "4.6.6",
            ),
            (
                "Cargo.toml",
                "[dependencies.tree-sitter]\nversion = \"0.26.12\"\n",
                "tree-sitter",
                "0.26.12",
            ),
            (
                "pyproject.toml",
                "[project]\ndependencies = [\n  \"fastapi[all]>=0.110,<0.111\",\n]\n",
                "fastapi",
                ">=0.110,<0.111",
            ),
            (
                "pyproject.toml",
                "[tool.poetry.dependencies]\ndjango = \"^5.0\"\n",
                "django",
                "^5.0",
            ),
            ("requirements.txt", "fastapi>=0.110  # pinned\n", "fastapi", ">=0.110"),
            ("Gemfile", "gem \"rails\", \"~> 7.1\", require: false\n", "rails", "~> 7.1"),
            (
                "go.mod",
                "require (\n\tgithub.com/spf13/cobra v1.8.0 // indirect\n)\n",
                "github.com/spf13/cobra",
                "v1.8.0",
            ),
        ];
        for (name, body, dependency, expected) in cases {
            let found = read(name, body, dependency).expect("the manifest reads");
            assert_eq!(
                found,
                Declared::Range(expected.to_string()),
                "{name} declaring {dependency}"
            );
        }
    }

    #[test]
    fn a_manifest_that_declares_something_else_is_absent_not_a_guess() {
        let found = read("package.json", r#"{"dependencies": {"react": "^18"}}"#, "tailwindcss")
            .expect("the manifest reads");
        assert_eq!(found, Declared::Absent);
    }

    /// `fastapi` must not be answered by `fastapi-users`, or a `when` reports
    /// on a package nobody asked about.
    #[test]
    fn a_longer_package_name_does_not_answer_for_a_shorter_one() {
        let found = read("requirements.txt", "fastapi-users>=13.0\n", "fastapi")
            .expect("the manifest reads");
        assert_eq!(found, Declared::Absent);
    }

    #[test]
    fn a_format_with_no_reader_says_so_rather_than_reporting_absent() {
        let found = read("build.gradle", "implementation 'com.google:guava:33.0'", "guava")
            .expect("the dispatch runs");
        assert_eq!(found, Declared::UnknownFormat);
    }

    #[test]
    fn a_manifest_that_does_not_parse_is_reported_rather_than_ignored() {
        let found = read("package.json", "{ not json", "tailwindcss").expect("the dispatch runs");
        assert!(matches!(found, Declared::Malformed(_)), "got {found:?}");
    }

    #[test]
    fn versions_parse_out_of_the_ranges_manifests_actually_write() {
        assert_eq!(Version::parse("^3.4.1"), Some(Version(vec![3, 4, 1])));
        assert_eq!(Version::parse(">=0.110,<0.111"), Some(Version(vec![0, 110])));
        assert_eq!(Version::parse("~> 7.1"), Some(Version(vec![7, 1])));
        assert_eq!(Version::parse("v1.8.0"), Some(Version(vec![1, 8, 0])));
        assert_eq!(Version::parse("*"), None);
        assert_eq!(Version::parse("https://github.com/o/r.git"), None);
    }

    #[test]
    fn a_version_that_stops_short_compares_as_zeroes() {
        assert_eq!(Version(vec![3]), Version(vec![3, 0, 0]));
        assert!(Version(vec![3, 4]) < Version(vec![3, 4, 1]));
        assert!(Version(vec![4]) > Version(vec![3, 99, 99]));
    }
}
