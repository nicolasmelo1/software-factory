//! L2 — the process running the checks is the runtime the repository pinned.
//!
//! A repository pins Node 20, the shell runs Node 18, and the first visible
//! disagreement is a regenerated lockfile or an artifact that differs for a
//! reason no diff contains. Every later finding is then a measurement taken
//! with the wrong instrument, so this check is a preflight: `run_all` runs it
//! before any other rule and stops on the first disagreement.
//!
//! Not the `toolchain` kind. That one asks whether CI invokes a hazard tool;
//! this one asks whether the process honours a version pin, and one name for
//! both questions would make either answer ambiguous.
//!
//! The version commands are fixed here and run as argv, never through a shell
//! and never taken from policy, so no `--allow-commands` is needed: nothing a
//! policy says can change what runs. Tests inject what the commands print
//! through [`Probe`], so the suite never depends on the runtimes of the host.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::Finding;
use crate::manifest::Version;
use anyhow::Result;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;

/// A runtime this check knows how to ask for its version.
pub struct Runtime {
    pub name: &'static str,
    /// Tried in order; the first executable that exists answers. `python` is
    /// what a pyenv shim resolves, `python3` is what a bare macOS carries.
    pub commands: &'static [&'static [&'static str]],
}

pub const NODE: Runtime = Runtime {
    name: "node",
    commands: &[&["node", "--version"]],
};
pub const PYTHON: Runtime = Runtime {
    name: "python",
    commands: &[&["python", "--version"], &["python3", "--version"]],
};
pub const RUST: Runtime = Runtime {
    name: "rustc",
    commands: &[&["rustc", "--version"]],
};
pub const GO: Runtime = Runtime {
    name: "go",
    commands: &[&["go", "version"]],
};
pub const RUBY: Runtime = Runtime {
    name: "ruby",
    commands: &[&["ruby", "--version"]],
};

/// What one version command had to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed {
    /// It ran and exited zero; the text is what it printed.
    Printed(String),
    /// No such executable on this process's `PATH`.
    Missing,
    /// It ran and failed; the text is its last words.
    Failed(String),
}

/// Answers "what does this command print here". The system one spawns the
/// process; tests hand in a table.
pub trait Probe {
    fn run(&self, argv: &[&str], root: &Path) -> Observed;
}

/// The real thing: argv, the repository as the working directory (so a
/// version manager's shim reads the same pin a developer's shell would), and
/// the two toolchain managers told not to download on our behalf.
pub struct SystemProbe;

impl Probe for SystemProbe {
    fn run(&self, argv: &[&str], root: &Path) -> Observed {
        let output = std::process::Command::new(argv[0])
            .args(&argv[1..])
            .current_dir(root)
            // `go version` under a newer `toolchain` line would otherwise
            // fetch and exec that toolchain, and rustup would install one.
            .env("GOTOOLCHAIN", "local")
            .env("RUSTUP_AUTO_INSTALL", "0")
            .output();
        match output {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Observed::Missing,
            Err(error) => Observed::Failed(error.to_string()),
            Ok(out) => {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                match out.status.success() {
                    true => Observed::Printed(text.trim().to_string()),
                    false => Observed::Failed(last_line(&text)),
                }
            }
        }
    }
}

fn last_line(text: &str) -> String {
    text.trim().lines().last().unwrap_or("").to_string()
}

/// One version a declaration states, for one runtime.
pub struct Pin {
    /// The file that states it, relative to the root.
    pub path: String,
    /// What the finding is about, stable across runs: the file and the
    /// runtime, or the directive where one file states two.
    pub key: String,
    pub runtime: &'static Runtime,
    /// Exactly as written, for the message.
    pub declared: String,
    pub reading: Reading,
}

/// What a declaration turned out to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// Alternatives, each a set of comparators that must all hold, in the
    /// small grammar [`crate::policy::satisfies`] reads. A bare `20.11.1` is
    /// an exact pin and a bare `20` is the series, as in every version manager.
    Requires(Vec<Vec<String>>),
    /// A statement that names no version: `system`, `lts/*`, `stable`. The
    /// repository did not say which one, so there is nothing to enforce. It
    /// is silent here and named by `L5.NO_INERT_RULE` if nothing else is left.
    NoVersion(String),
    /// Syntax this check cannot interpret. A finding, never a guessed match.
    Malformed(String),
}

impl Pin {
    fn new(path: &str, runtime: &'static Runtime, declared: &str, reading: Reading) -> Pin {
        Pin {
            path: path.to_string(),
            key: format!("{path}:{}", runtime.name),
            runtime,
            declared: declared.to_string(),
            reading,
        }
    }
}

/// Reads what one declaration file says, from its body.
type Reader = fn(&str) -> Vec<Pin>;

/// Every declaration this check reads, in the order it reports them. One
/// table, so the formats a test holds to a repair cannot drift from the ones
/// the check reads.
pub const READERS: &[(&str, Reader)] = &[
    (".nvmrc", |body| {
        first_line_pin(".nvmrc", &NODE, body, node_alias)
            .into_iter()
            .collect()
    }),
    (".node-version", |body| {
        first_line_pin(".node-version", &NODE, body, node_alias)
            .into_iter()
            .collect()
    }),
    (".python-version", |body| {
        first_line_pin(".python-version", &PYTHON, body, python_alias)
            .into_iter()
            .collect()
    }),
    ("rust-toolchain.toml", |body| {
        rust_toolchain(body).into_iter().collect()
    }),
    (".tool-versions", tool_versions),
    ("go.mod", go_mod),
    ("package.json", |body| engines(body).into_iter().collect()),
];

/// Every pin the root declares, in a stable order.
pub fn declarations(root: &Path) -> Vec<Pin> {
    READERS
        .iter()
        .filter_map(|(name, reader)| read(root, name).map(|body| reader(&body)))
        .flatten()
        .collect()
}

fn read(root: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(root.join(name)).ok()
}

/// A bare release number, with an optional leading `v`: `20`, `3.12`, `v1.80.0`.
fn bare(text: &str) -> Option<String> {
    let pattern = Regex::new(r"^v?(\d+(?:\.\d+){0,2})$").expect("the pattern compiles");
    pattern.captures(text).map(|c| c[1].to_string())
}

/// `.nvmrc`, `.node-version` and `.python-version`: the first line that is
/// not a comment is the version the shell activates.
fn first_line_pin(
    path: &str,
    runtime: &'static Runtime,
    body: &str,
    alias: fn(&str) -> bool,
) -> Option<Pin> {
    let line = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))?;
    Some(Pin::new(path, runtime, line, read_single(line, alias)))
}

fn read_single(text: &str, alias: fn(&str) -> bool) -> Reading {
    if let Some(version) = bare(text) {
        return Reading::Requires(vec![vec![version]]);
    }
    if text == "system" || alias(text) {
        return Reading::NoVersion(format!("`{text}` names no version"));
    }
    Reading::Malformed(format!("`{text}` is not a version this check can read"))
}

fn node_alias(text: &str) -> bool {
    text.starts_with("lts/") || ["node", "stable", "latest", "current"].contains(&text)
}

/// pyenv names other interpreters (`pypy3.10-7.3.12`, `miniconda3-latest`)
/// the same way it names CPython. Those state a version of something `python
/// --version` does not report comparably, so there is nothing to hold it to.
fn python_alias(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
}

/// `channel = "…"` under `[toolchain]`. A file with no channel pins nothing:
/// it only names components.
fn rust_toolchain(body: &str) -> Option<Pin> {
    let channel = Regex::new(r#"^\s*channel\s*=\s*"([^"]*)""#).expect("the pattern compiles");
    let mut section = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = trimmed.to_string();
            continue;
        }
        let Some(found) = channel.captures(line).filter(|_| section == "[toolchain]") else {
            continue;
        };
        let value = found[1].to_string();
        return Some(Pin::new(
            "rust-toolchain.toml",
            &RUST,
            &value,
            read_single(&value, rust_channel),
        ));
    }
    None
}

fn rust_channel(text: &str) -> bool {
    ["stable", "beta", "nightly"]
        .iter()
        .any(|channel| text == *channel || text.starts_with(&format!("{channel}-")))
}

/// asdf and mise: one runtime per line, the first version listed is the one
/// the shim runs. Each supported runtime is its own pin, so a matching one
/// cannot hide a mismatch on the next line. Tools this check has no adapter
/// for are not a statement about anything it can measure.
fn tool_versions(body: &str) -> Vec<Pin> {
    let mut pins = Vec::new();
    for line in body.lines() {
        let content = line.split('#').next().unwrap_or("").trim();
        let mut words = content.split_whitespace();
        let Some(tool) = words.next() else { continue };
        let Some(runtime) = asdf_runtime(tool) else {
            continue;
        };
        let reading = match words.next() {
            Some(version) => read_single(version, |v| {
                v.starts_with("ref:") || v.starts_with("path:") || v.starts_with("latest")
            }),
            None => Reading::Malformed(format!("`{tool}` is listed with no version")),
        };
        pins.push(Pin::new(".tool-versions", runtime, content, reading));
    }
    pins
}

fn asdf_runtime(tool: &str) -> Option<&'static Runtime> {
    match tool {
        "nodejs" | "node" => Some(&NODE),
        "python" => Some(&PYTHON),
        "rust" => Some(&RUST),
        "golang" | "go" => Some(&GO),
        "ruby" => Some(&RUBY),
        _ => None,
    }
}

/// `go 1.22` is a minimum, in Go's own semantics, and so is a `toolchain
/// go1.22.3` line. Each is checked on its own.
fn go_mod(body: &str) -> Vec<Pin> {
    let directive =
        Regex::new(r"^\s*(go|toolchain)\s+(\S+)\s*(?://.*)?$").expect("the pattern compiles");
    let mut pins = Vec::new();
    for line in body.lines() {
        let Some(found) = directive.captures(line) else {
            continue;
        };
        let value = found[2].to_string();
        let number = value.strip_prefix("go").unwrap_or(&value);
        let (declared, reading) = match (&found[1], number) {
            ("toolchain", "default") => (
                value.clone(),
                Reading::NoVersion("`toolchain default` names no version".into()),
            ),
            (_, number) if bare_go(number) => (
                format!(">={number}"),
                Reading::Requires(vec![vec![format!(">={number}")]]),
            ),
            _ => (
                value.clone(),
                Reading::Malformed(format!("`{value}` is not a Go version")),
            ),
        };
        let mut pin = Pin::new("go.mod", &GO, &declared, reading);
        pin.key = format!("go.mod:{}", &found[1]);
        pins.push(pin);
    }
    pins
}

/// `1.22`, `1.22.3`, `1.21rc2`: Go's release numbers, pre-releases included.
fn bare_go(number: &str) -> bool {
    Regex::new(r"^\d+(\.\d+){1,2}((rc|beta)\d+)?$")
        .expect("the pattern compiles")
        .is_match(number)
}

/// `engines.node` in `package.json`, in npm's range grammar.
fn engines(body: &str) -> Option<Pin> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
        return Some(Pin::new(
            "package.json",
            &NODE,
            "",
            Reading::Malformed("package.json did not parse".into()),
        ));
    };
    let value = json.get("engines")?.get("node")?;
    let Some(range) = value.as_str() else {
        return Some(Pin::new(
            "package.json",
            &NODE,
            &value.to_string(),
            Reading::Malformed("`engines.node` is not a string".into()),
        ));
    };
    Some(Pin::new("package.json", &NODE, range, npm_range(range)))
}

/// npm ranges: `||` between alternatives, whitespace between comparators that
/// must all hold, `a - b` for an inclusive span, `x` and `*` as wildcards.
pub fn npm_range(range: &str) -> Reading {
    let trimmed = range.trim();
    if trimmed.is_empty() || trimmed == "*" || trimmed == "x" {
        return Reading::NoVersion(format!("`{range}` accepts every version"));
    }
    let mut alternatives = Vec::new();
    for alternative in trimmed.split("||") {
        match comparators(alternative) {
            Some(set) => alternatives.push(set),
            None => {
                return Reading::Malformed(format!(
                    "`{range}` is not an npm range this check can read"
                ));
            }
        }
    }
    Reading::Requires(alternatives)
}

fn comparators(alternative: &str) -> Option<Vec<String>> {
    let glued = Regex::new(r"(<=|>=|<|>|=|\^|~)\s+").expect("the pattern compiles");
    let alternative = glued.replace_all(alternative.trim(), "$1");
    let words: Vec<&str> = alternative.split_whitespace().collect();
    if let [low, "-", high] = words.as_slice() {
        return [low, high]
            .iter()
            .all(|end| bare(end).is_some())
            .then(|| vec![format!(">={low}"), format!("<={high}")]);
    }
    let comparator = Regex::new(r"^(<=|>=|<|>|=|\^|~)?v?\d+(\.(\d+|x|X|\*)){0,2}$")
        .expect("the pattern compiles");
    match !words.is_empty() && words.iter().all(|word| comparator.is_match(word)) {
        true => Some(
            words
                .iter()
                .map(|word| word.replace(['x', 'X', '*'], ""))
                .collect(),
        ),
        false => None,
    }
}

/// Whether `observed` falls in what a reading requires.
fn holds(alternatives: &[Vec<String>], observed: &Version) -> bool {
    alternatives.iter().any(|set| {
        set.iter()
            .all(|comparator| crate::policy::satisfies(comparator.trim_end_matches('.'), observed))
    })
}

/// Every finding the pins earn. Empty means every pin that states a version
/// agrees with the process that would run it.
pub fn check(rule: &Rule, root: &Path, probe: &dyn Probe) -> Vec<Finding> {
    let mut asked: BTreeMap<&str, (String, Observed)> = BTreeMap::new();
    let mut findings = Vec::new();
    for pin in declarations(root) {
        let alternatives = match &pin.reading {
            Reading::NoVersion(_) => continue,
            Reading::Malformed(why) => {
                findings.push(malformed(rule, &pin, why));
                continue;
            }
            Reading::Requires(alternatives) => alternatives,
        };
        let (command, observed) = asked
            .entry(pin.runtime.name)
            .or_insert_with(|| ask(pin.runtime, root, probe))
            .clone();
        findings.extend(compare(rule, &pin, alternatives, &command, &observed));
    }
    findings
}

/// The first command whose executable exists, and what it said.
fn ask(runtime: &Runtime, root: &Path, probe: &dyn Probe) -> (String, Observed) {
    for argv in runtime.commands {
        let observed = probe.run(argv, root);
        if observed != Observed::Missing {
            return (argv.join(" "), observed);
        }
    }
    let tried: Vec<&str> = runtime.commands.iter().map(|argv| argv[0]).collect();
    (tried.join(", "), Observed::Missing)
}

fn compare(
    rule: &Rule,
    pin: &Pin,
    alternatives: &[Vec<String>],
    command: &str,
    observed: &Observed,
) -> Option<Finding> {
    let wanted = format!("{} {}", pin.runtime.name, pin.declared);
    let (message, actual) = match observed {
        Observed::Missing => (
            format!(
                "`{}` pins {wanted}, and no executable for it is on this process's PATH",
                pin.path
            ),
            format!("not found (tried: {command})"),
        ),
        Observed::Failed(why) => (
            format!(
                "`{}` pins {wanted}, and `{command}` failed instead of reporting a version",
                pin.path
            ),
            why.clone(),
        ),
        Observed::Printed(text) => match Version::parse(text) {
            Some(version) if holds(alternatives, &version) => return None,
            Some(_) => (
                format!(
                    "`{}` pins {wanted}, but `{command}` in this repository reports `{text}`",
                    pin.path
                ),
                text.clone(),
            ),
            None => (
                format!(
                    "`{}` pins {wanted}, and `{command}` printed no version",
                    pin.path
                ),
                text.clone(),
            ),
        },
    };
    Some(
        Finding::new(&rule.id, rule.severity, &pin.path, &pin.key, message)
            .expected(wanted)
            .actual(actual),
    )
}

fn malformed(rule: &Rule, pin: &Pin, why: &str) -> Finding {
    Finding::new(
        &rule.id,
        rule.severity,
        &pin.path,
        &pin.key,
        format!(
            "`{}` could not be read as a {} version: {why}",
            pin.path, pin.runtime.name
        ),
    )
    .expected("a release number such as 20.11.1, or a range in the declaration's own grammar")
    .actual(pin.declared.clone())
}

pub fn run(rule: &Rule, ctx: &Ctx) -> Result<Vec<Finding>> {
    Ok(check(rule, ctx.root, &SystemProbe))
}

/// Why an enabled runtime-pin rule can never fire here, if it cannot: the
/// root states no version for any runtime this check can ask.
pub fn inert_reason(root: &Path) -> Option<String> {
    let pins = declarations(root);
    if pins
        .iter()
        .any(|pin| !matches!(pin.reading, Reading::NoVersion(_)))
    {
        return None;
    }
    let said: Vec<String> = pins
        .iter()
        .filter_map(|pin| match &pin.reading {
            Reading::NoVersion(why) => Some(format!("`{}`: {why}", pin.path)),
            _ => None,
        })
        .collect();
    Some(match said.is_empty() {
        true => "no runtime pin at the root: it has no version to hold any process to".to_string(),
        false => format!("no pin at the root states a version ({})", said.join("; ")),
    })
}

/// A probe that answers from a table: argv joined by spaces, to what it
/// printed. Anything not in the table is a missing executable.
#[cfg(test)]
pub struct Answers(pub BTreeMap<String, Observed>);

#[cfg(test)]
impl Answers {
    pub fn printing(pairs: &[(&str, &str)]) -> Answers {
        Answers(
            pairs
                .iter()
                .map(|(argv, text)| (argv.to_string(), Observed::Printed(text.to_string())))
                .collect(),
        )
    }
}

#[cfg(test)]
impl Probe for Answers {
    fn run(&self, argv: &[&str], _root: &Path) -> Observed {
        self.0
            .get(&argv.join(" "))
            .cloned()
            .unwrap_or(Observed::Missing)
    }
}

/// A fresh directory per call. Tests run in parallel, so the name carries a
/// counter as well as the process id.
#[cfg(test)]
pub fn scratch(tag: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("sf-pin-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, body) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a file has a parent"))
            .expect("the directory is created");
        std::fs::write(path, body).expect("the file is written");
    }
    std::fs::create_dir_all(&root).expect("the root exists");
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::checks::{Ctx, run_all_with};
    use crate::policy::Policy;
    use crate::ratchet::Ratchet;
    use crate::scan;

    const RULE: &str = "L2.RUNNING_TOOLCHAIN_MATCHES_THE_PIN";
    const BARE_NOQA: &str = concat!("#", " noqa");

    fn rule() -> Rule {
        Catalog::builtin()
            .expect("the built-in catalog loads")
            .get(RULE)
            .expect("the rule ships")
            .clone()
    }

    fn findings(files: &[(&str, &str)], answers: &[(&str, &str)]) -> Vec<Finding> {
        let root = scratch("unit", files);
        check(&rule(), &root, &Answers::printing(answers))
    }

    /// The whole run, pin rule and one ordinary rule that has something to
    /// say, so the test can see whether the ordinary one was reached.
    fn run_everything(answers: &[(&str, &str)]) -> Vec<Finding> {
        let noqa = format!("import os  {BARE_NOQA}\n");
        let root = scratch(
            "preflight",
            &[(".nvmrc", "20.11.1\n"), ("src/app.py", &noqa)],
        );
        let policy: Policy = serde_yaml::from_str(&format!(
            "version: 1\nproject:\n  name: pinned\n  languages: [python]\nrules:\n  L1.NO_BLANKET_SUPPRESSION:\n    enabled: true\n  {RULE}:\n    enabled: true\n"
        ))
        .expect("the policy parses");
        let catalog = Catalog::builtin().expect("the built-in catalog loads");
        let files = scan::walk(&root, &policy).expect("the scratch repo scans");
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
            overlay: None,
        };
        run_all_with(&ctx, &Answers::printing(answers)).expect("the run completes")
    }

    #[test]
    fn a_mismatched_pin_stops_the_run_before_any_ordinary_rule() {
        let found = run_everything(&[("node --version", "v18.19.0")]);
        assert_eq!(found.len(), 1, "{found:?}");
        let pin = &found[0];
        assert_eq!(pin.rule, RULE);
        assert_eq!(pin.location, ".nvmrc");
        assert!(pin.message.contains("node --version"), "{}", pin.message);
        assert_eq!(pin.expected.as_deref(), Some("node 20.11.1"));
        assert_eq!(pin.actual.as_deref(), Some("v18.19.0"));
    }

    /// The other half of the stop: with the runtime right, the ordinary rule
    /// runs and reports what it always would have.
    #[test]
    fn a_matching_pin_lets_every_other_rule_run() {
        let found = run_everything(&[("node --version", "v20.11.1")]);
        assert!(found.iter().all(|f| f.rule != RULE), "{found:?}");
        assert!(
            found.iter().any(|f| f.rule == "L1.NO_BLANKET_SUPPRESSION"),
            "{found:?}"
        );
    }

    #[test]
    fn an_exact_pin_that_matches_passes() {
        let found = findings(
            &[(".python-version", "3.12.1\n")],
            &[("python --version", "Python 3.12.1")],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_satisfied_range_passes_and_an_unsatisfied_one_does_not() {
        let manifest = r#"{"engines": {"node": ">=18 <21"}}"#;
        assert!(
            findings(
                &[("package.json", manifest)],
                &[("node --version", "v20.11.1")]
            )
            .is_empty()
        );
        assert_eq!(
            findings(
                &[("package.json", manifest)],
                &[("node --version", "v21.0.0")]
            )
            .len(),
            1
        );
    }

    #[test]
    fn a_series_pin_accepts_any_release_in_the_series() {
        assert!(
            findings(
                &[(".node-version", "20\n")],
                &[("node --version", "v20.18.0")]
            )
            .is_empty()
        );
        assert_eq!(
            findings(
                &[(".node-version", "20\n")],
                &[("node --version", "v22.1.0")]
            )
            .len(),
            1
        );
    }

    #[test]
    fn a_go_directive_is_a_minimum() {
        let module = "module example.com/app\n\ngo 1.22\n";
        assert!(
            findings(
                &[("go.mod", module)],
                &[("go version", "go version go1.23.2 linux/amd64")]
            )
            .is_empty()
        );
        assert_eq!(
            findings(
                &[("go.mod", module)],
                &[("go version", "go version go1.21.5 linux/amd64")]
            )
            .len(),
            1
        );
    }

    #[test]
    fn a_malformed_declaration_is_its_own_finding() {
        let found = findings(&[(".nvmrc", "twenty\n")], &[("node --version", "v20.11.1")]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].message.contains("could not be read"),
            "{}",
            found[0].message
        );
        assert_eq!(found[0].actual.as_deref(), Some("twenty"));
    }

    #[test]
    fn a_missing_executable_is_named_as_missing_not_as_a_mismatch() {
        let found = findings(
            &[("rust-toolchain.toml", "[toolchain]\nchannel = \"1.80.0\"\n")],
            &[],
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].message.contains("no executable"),
            "{}",
            found[0].message
        );
        assert_eq!(found[0].actual.as_deref(), Some("not found (tried: rustc)"));
    }

    #[test]
    fn a_failing_version_command_is_named_as_failing() {
        let mut answers = BTreeMap::new();
        answers.insert(
            "go version".to_string(),
            Observed::Failed("go: cannot find GOROOT".into()),
        );
        let root = scratch("failing", &[("go.mod", "module m\n\ngo 1.22\n")]);
        let found = check(&rule(), &root, &Answers(answers));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("failed"), "{}", found[0].message);
    }

    #[test]
    fn python3_answers_when_there_is_no_python() {
        let found = findings(
            &[(".python-version", "3.12\n")],
            &[("python3 --version", "Python 3.12.4")],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_repository_with_no_pin_is_not_told_what_to_use() {
        let found = findings(
            &[("README.md", "# nothing pinned\n")],
            &[("node --version", "v18.0.0")],
        );
        assert!(found.is_empty(), "{found:?}");
        let root = scratch("none", &[("README.md", "# nothing pinned\n")]);
        assert!(inert_reason(&root).is_some());
    }

    /// `lts/*` and `stable` said a runtime and not a version, so there is
    /// nothing to hold the process to; the only place that shows is the
    /// inertness rule, which names what it read.
    #[test]
    fn a_pin_that_names_no_version_is_silent_and_says_why_it_is_inert() {
        let files = [
            (".nvmrc", "lts/*\n"),
            ("rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n"),
        ];
        assert!(findings(&files, &[]).is_empty());
        let reason = inert_reason(&scratch("alias", &files)).expect("nothing states a version");
        assert!(
            reason.contains("lts/*") && reason.contains("stable"),
            "{reason}"
        );
    }

    #[test]
    fn npm_ranges_read_in_npm_grammar() {
        let observed = Version::parse("20.11.1").expect("a version");
        for accepted in [
            "^20",
            "~20.11",
            "20.x",
            ">= 18",
            "16 || >=20",
            "18 - 20.11.1",
            "v20.11.1",
        ] {
            let Reading::Requires(alternatives) = npm_range(accepted) else {
                panic!("`{accepted}` should read as a range");
            };
            assert!(
                holds(&alternatives, &observed),
                "`{accepted}` should accept 20.11.1"
            );
        }
        for rejected in ["^18", "<20", "~20.10"] {
            let Reading::Requires(alternatives) = npm_range(rejected) else {
                panic!("`{rejected}` should read as a range");
            };
            assert!(
                !holds(&alternatives, &observed),
                "`{rejected}` should reject 20.11.1"
            );
        }
        assert!(matches!(npm_range("latest"), Reading::Malformed(_)));
        assert!(matches!(npm_range("*"), Reading::NoVersion(_)));
    }
}
