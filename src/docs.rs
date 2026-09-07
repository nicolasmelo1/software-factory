//! Deterministic, marker-scoped documentation generation.
//!
//! Anything a machine can read out of this repository is rendered here and
//! never typed into a page by hand. Nothing in the output depends on the
//! clock or on the machine that produced it, which is what lets `--check`
//! compare a fresh render against what is committed.

use crate::catalog::{Catalog, CheckKind, Layer, Rule};
use crate::init;
use crate::interview::{self, Interview};
use crate::policy::{self, Policy};
use crate::ratchet::Ratchet;
use anyhow::{Result, bail};
use clap::CommandFactory;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Every block this binary renders. A block nothing places is an error: a
/// fact the documentation holds and never shows is worth as little as one
/// that has gone out of date.
const BLOCKS: &[&str] = &[
    "cli-detail",
    "cli-global",
    "cli-index",
    "hazard-tools",
    "interview-tree",
    "language-coverage",
    "language-grammars",
    "layer-index",
    "rules-summary",
    "skills-index",
    "templates-index",
];

const LAYERS: [Layer; 7] =
    [Layer::L0, Layer::L1, Layer::L2, Layer::L3, Layer::L4, Layer::L5, Layer::L6];

/// Flags every subcommand carries. They are documented once in prose rather
/// than in every generated row.
const UNIVERSAL_FLAGS: [&str; 3] = ["help", "version", "root"];

pub struct RenderedPage {
    pub path: PathBuf,
    pub content: String,
}

pub fn render(root: &Path, catalog: &Catalog) -> Result<Vec<RenderedPage>> {
    let generated = blocks(root, catalog)?;
    let mut found = BTreeSet::new();
    let mut rendered = Vec::new();
    for path in markdown_pages(root)? {
        let original = std::fs::read_to_string(root.join(&path))?;
        let (content, names) = replace_blocks(&original, &generated)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        found.extend(names);
        rendered.push(RenderedPage { path, content });
    }
    let missing: Vec<&str> = BLOCKS.iter().copied().filter(|name| !found.contains(*name)).collect();
    if !missing.is_empty() {
        bail!("generated block(s) have no placement: {}", missing.join(", "));
    }
    Ok(rendered)
}

pub fn check(root: &Path, catalog: &Catalog) -> Result<Vec<PathBuf>> {
    let mut changed = Vec::new();
    for page in render(root, catalog)? {
        let current = std::fs::read_to_string(root.join(&page.path))?;
        if current != page.content {
            changed.push(page.path);
        }
    }
    let (rules_path, rules_content) = init::rules_document_content(root, catalog)?;
    if std::fs::read_to_string(&rules_path).unwrap_or_default() != rules_content {
        changed.push(rules_path);
    }
    Ok(changed)
}

pub fn apply(root: &Path, catalog: &Catalog) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for page in render(root, catalog)? {
        let path = root.join(&page.path);
        let current = std::fs::read_to_string(&path)?;
        if current != page.content {
            std::fs::write(&path, page.content)?;
            written.push(page.path);
        }
    }
    Ok(written)
}

fn blocks(root: &Path, catalog: &Catalog) -> Result<BTreeMap<&'static str, String>> {
    let policy = Policy::load(root)?;
    let ratchet = Ratchet::load(root)?;
    let interview = Interview::load()?;
    let mut out = BTreeMap::new();
    out.insert("cli-detail", cli_detail());
    out.insert("cli-global", cli_global());
    out.insert("cli-index", cli_index());
    out.insert("hazard-tools", hazard_tools(catalog));
    out.insert("interview-tree", interview_tree(&interview));
    out.insert("language-coverage", language_coverage(catalog));
    out.insert("language-grammars", language_grammars());
    out.insert("layer-index", layer_index(catalog, &policy));
    out.insert("rules-summary", rules_summary(root, catalog, &policy, &ratchet));
    out.insert("skills-index", skills_index());
    out.insert("templates-index", templates_index());
    Ok(out)
}

fn markdown_pages(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(root.join("docs")) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|s| s.to_str()) == Some("md")
        {
            paths.push(entry.path().strip_prefix(root)?.to_path_buf());
        }
    }
    if root.join("README.md").is_file() {
        paths.push(PathBuf::from("README.md"));
    }
    paths.sort();
    Ok(paths)
}

/// A marker inside a fenced code block is documentation showing the form, so
/// the page explaining this mechanism can quote it. Only markers in prose
/// are replaced, which is the same distinction `L4.CLAIM_CITES_ITS_EVIDENCE`
/// draws about its own marker.
fn replace_blocks(
    input: &str,
    generated: &BTreeMap<&str, String>,
) -> Result<(String, BTreeSet<String>)> {
    let mut out = String::with_capacity(input.len());
    let mut found = BTreeSet::new();
    let mut lines = input.split_inclusive('\n');
    let mut fenced = false;
    while let Some(line) = lines.next() {
        let trimmed = line.trim_end_matches(['\r', '\n']).trim();
        if trimmed.starts_with("```") {
            fenced = !fenced;
        }
        let opened = if fenced { None } else { block_name(trimmed) };
        let Some(name) = opened else {
            out.push_str(line);
            continue;
        };
        if !generated.contains_key(name) {
            bail!("unknown generated block `{name}`");
        }
        if !found.insert(name.to_string()) {
            bail!("generated block `{name}` is placed more than once");
        }
        out.push_str(line);
        close_block(&mut out, &mut lines, name, &generated[name])?;
    }
    Ok((out, found))
}

fn block_name(trimmed: &str) -> Option<&str> {
    trimmed.strip_prefix("<!-- sf:generated ").and_then(|s| s.strip_suffix(" -->"))
}

fn close_block<'a>(
    out: &mut String,
    lines: &mut impl Iterator<Item = &'a str>,
    name: &str,
    content: &str,
) -> Result<()> {
    let end = format!("<!-- sf:end {name} -->");
    for inner in lines {
        if inner.trim_end_matches(['\r', '\n']).trim() == end {
            out.push_str(&format!("{content}\n"));
            out.push_str(inner);
            return Ok(());
        }
    }
    bail!("generated block `{name}` has no end marker")
}

fn enabled_ids(policy: &Policy) -> BTreeSet<String> {
    policy
        .instances()
        .into_iter()
        .map(|(_, id)| policy::base_rule_id(&id).to_string())
        .collect()
}

fn rules_summary(root: &Path, catalog: &Catalog, policy: &Policy, ratchet: &Ratchet) -> String {
    let shipped = catalog.rules.len();
    let enabled = enabled_ids(policy);
    let fixtures = enabled
        .iter()
        .filter(|id| root.join(policy::FIXTURES_DIR).join(id).is_dir())
        .count();
    let frozen: usize = ratchet.rules.values().map(|rule| rule.allow.len()).sum();
    let dates: Vec<String> = ratchet
        .rules
        .iter()
        .map(|(id, rule)| format!("`{id}` by {}", rule.review_by))
        .collect();
    let debt = if dates.is_empty() {
        "Nothing is frozen.".to_string()
    } else {
        format!("Frozen, with a date the build fails on: {}.", dates.join(", "))
    };
    format!(
        "**{shipped} rules shipped**, {} enabled here, {} switched off, {fixtures} carrying a \
         mutation fixture, {frozen} violations frozen.\n\n{debt}",
        enabled.len(),
        shipped.saturating_sub(enabled.len()),
    )
}

fn layer_index(catalog: &Catalog, policy: &Policy) -> String {
    let enabled = enabled_ids(policy);
    let mut out = String::from("| | Layer | What it checks | Shipped | Enabled here |\n");
    out.push_str("| :-- | :-- | :-- | --: | --: |\n");
    for layer in LAYERS {
        let rules: Vec<&Rule> =
            catalog.rules.values().filter(|rule| rule.layer == layer).collect();
        let on = rules.iter().filter(|rule| enabled.contains(&rule.id)).count();
        let title = init::layer_title(layer);
        let (name, about) = title.split_once(": ").unwrap_or((title, ""));
        out.push_str(&format!(
            "| **{}** | {name} | {about} | {} | {on} |\n",
            layer.as_str(),
            rules.len(),
        ));
    }
    out
}

fn subcommands() -> Vec<clap::Command> {
    let cli = crate::Cli::command();
    let mut commands: Vec<clap::Command> = cli.get_subcommands().cloned().collect();
    commands.sort_by_key(|command| command.get_name().to_string());
    commands
}

fn about_of(command: &clap::Command) -> String {
    command.get_about().map(|about| about.to_string()).unwrap_or_default()
}

fn cli_index() -> String {
    let mut out = String::from("| Command | What it does |\n| :-- | :-- |\n");
    for command in subcommands() {
        let name = command.get_name();
        out.push_str(&format!(
            "| [`sf {name}`](#sf-{name}) | {} |\n",
            about_of(&command).trim_end_matches('.'),
        ));
    }
    out
}

fn cli_global() -> String {
    let mut out = String::from("| Option | What it does |\n| :-- | :-- |\n");
    for arg in crate::Cli::command().get_arguments() {
        let Some(long) = arg.get_long() else { continue };
        if long == "help" || long == "version" {
            continue;
        }
        out.push_str(&format!("| `{}` | {} |\n", usage_of(arg), help_of(arg)));
    }
    out
}

fn cli_detail() -> String {
    let mut out = String::new();
    for command in subcommands() {
        let name = command.get_name();
        out.push_str(&format!("### `sf {name}`\n\n{}\n\n", about_of(&command)));
        out.push_str(&format!("```sh\nsf {}\n```\n\n", invocation(&command)));
        let flags = flag_table(&command);
        out.push_str(&flags.unwrap_or_else(|| {
            "No flags of its own; the global options above apply.\n".to_string()
        }));
        out.push('\n');
    }
    out.trim_end().to_string()
}

fn invocation(command: &clap::Command) -> String {
    let mut parts = vec![command.get_name().to_string()];
    for arg in command.get_positionals() {
        parts.push(format!("<{}>", arg.get_id().as_str().to_uppercase()));
    }
    if flag_table(command).is_some() {
        parts.push("[options]".to_string());
    }
    parts.join(" ")
}

fn flag_table(command: &clap::Command) -> Option<String> {
    let mut rows = String::new();
    for arg in command.get_arguments() {
        let Some(long) = arg.get_long() else { continue };
        if UNIVERSAL_FLAGS.contains(&long) {
            continue;
        }
        rows.push_str(&format!(
            "| `{}` | {} | {} |\n",
            usage_of(arg),
            help_of(arg),
            default_of(arg),
        ));
    }
    if rows.is_empty() {
        return None;
    }
    Some(format!("| Flag | What it does | Default |\n| :-- | :-- | :-- |\n{rows}"))
}

fn takes_a_value(arg: &clap::Arg) -> bool {
    matches!(arg.get_action(), clap::ArgAction::Set | clap::ArgAction::Append)
}

fn usage_of(arg: &clap::Arg) -> String {
    let long = arg.get_long().unwrap_or_default();
    if takes_a_value(arg) {
        return format!("--{long} <{}>", arg.get_id().as_str().to_uppercase());
    }
    format!("--{long}")
}

fn help_of(arg: &clap::Arg) -> String {
    let help = arg.get_help().map(|help| help.to_string()).unwrap_or_default();
    let text = help.replace('\n', " ");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match text.split_once(". ") {
        Some((first, _)) => format!("{first}."),
        None => text,
    }
}

fn default_of(arg: &clap::Arg) -> String {
    let values: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|value| format!("`{}`", value.to_string_lossy()))
        .collect();
    if !values.is_empty() {
        return values.join(", ");
    }
    if takes_a_value(arg) { "none".to_string() } else { "off".to_string() }
}

fn query_languages(rule: &Rule) -> BTreeSet<String> {
    match &rule.check {
        CheckKind::Shape { languages } => languages.keys().cloned().collect(),
        CheckKind::Forwarder { languages } => languages.keys().cloned().collect(),
        CheckKind::Nested { languages } => languages.keys().cloned().collect(),
        _ => BTreeSet::new(),
    }
}

fn language_coverage(catalog: &Catalog) -> String {
    let mut out = String::from("| Rule | Languages with a query |\n| :-- | :-- |\n");
    for rule in catalog.rules.values() {
        let languages = query_languages(rule);
        if languages.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "| `{}` | {} |\n",
            rule.id,
            languages.into_iter().collect::<Vec<_>>().join(", "),
        ));
    }
    out
}

fn language_grammars() -> String {
    let mut out = String::from("| Grammar | Files it reads |\n| :-- | :-- |\n");
    for (lang, extensions) in crate::lang::GRAMMARS {
        let globs: Vec<String> =
            extensions.iter().map(|extension| format!("`*.{extension}`")).collect();
        out.push_str(&format!("| {} | {} |\n", lang.name(), globs.join(", ")));
    }
    out
}

fn tools_of(rule: &Rule) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let Some(mapping) = rule.defaults.get("tools").and_then(|value| value.as_mapping()) else {
        return out;
    };
    for (language, tools) in mapping {
        let Some(language) = language.as_str() else { continue };
        let listed: Vec<String> = tools
            .as_sequence()
            .map(|values| values.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        out.insert(language.to_string(), listed);
    }
    out
}

fn hazard_tools(catalog: &Catalog) -> String {
    let hazards: Vec<(&Rule, BTreeMap<String, Vec<String>>)> = catalog
        .rules
        .values()
        .filter(|rule| matches!(rule.check, CheckKind::Toolchain))
        .map(|rule| (rule, tools_of(rule)))
        .collect();
    let languages: BTreeSet<String> =
        hazards.iter().flat_map(|(_, tools)| tools.keys().cloned()).collect();
    let header: Vec<&str> = languages.iter().map(String::as_str).collect();
    let mut out = format!("| Concern | {} |\n", header.join(" | "));
    out.push_str(&format!("| :-- |{}\n", " :-- |".repeat(header.len())));
    for (rule, tools) in &hazards {
        let cells: Vec<String> = languages
            .iter()
            .map(|language| match tools.get(language) {
                Some(listed) if !listed.is_empty() => listed.join(", "),
                _ => "no tool listed".to_string(),
            })
            .collect();
        out.push_str(&format!("| {} | {} |\n", rule.title, cells.join(" | ")));
    }
    out
}

fn templates_index() -> String {
    let mut out = String::new();
    out.push_str("| Template | Rule it writes | What that rule requires | Filled in with |\n");
    out.push_str("| :-- | :-- | :-- | :-- |\n");
    for template in interview::template_docs() {
        let placeholders: Vec<String> =
            template.placeholders.iter().map(|name| format!("`{name}`")).collect();
        let filled = if placeholders.is_empty() {
            "nothing; it ships as written".to_string()
        } else {
            placeholders.join(", ")
        };
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {filled} |\n",
            template.name, template.rule_id, template.title,
        ));
    }
    out
}

fn skills_index() -> String {
    let mut out = String::from("| Skill | What it is for |\n| :-- | :-- |\n");
    for (name, body) in crate::skills::SKILLS {
        out.push_str(&format!("| `/{name}` | {} |\n", first_sentence(description_of(body))));
    }
    out
}

fn description_of(body: &str) -> &str {
    body.lines()
        .find_map(|line| line.strip_prefix("description: "))
        .unwrap_or_default()
        .trim()
}

fn first_sentence(text: &str) -> String {
    match text.split_once(". ") {
        Some((first, _)) => format!("{first}."),
        None => text.to_string(),
    }
}

fn interview_tree(interview: &Interview) -> String {
    let mut out = String::new();
    for decision in &interview.decisions {
        out.push_str(&format!("### `{}` — {}\n\n", decision.id, decision.question));
        out.push_str(&format!("{}\n\n", wrapped(decision.why.trim())));
        if let Some(condition) = asked_when(decision) {
            out.push_str(&format!("{condition}\n\n"));
        }
        out.push_str(&answers_table(decision));
        out.push('\n');
    }
    out.trim_end().to_string()
}

/// Wrap a generated paragraph the way the hand-written prose beside it is
/// wrapped, so a page reads the same whoever produced the line.
fn wrapped(text: &str) -> String {
    let mut out = String::new();
    let mut column = 0;
    for word in text.split_whitespace() {
        if column > 0 && column + 1 + word.len() > 78 {
            out.push('\n');
            column = 0;
        } else if column > 0 {
            out.push(' ');
            column += 1;
        }
        out.push_str(word);
        column += word.len();
    }
    out
}

fn asked_when(decision: &interview::Decision) -> Option<String> {
    if decision.depends_on.is_empty() {
        return None;
    }
    let conditions: Vec<String> = decision
        .depends_on
        .iter()
        .map(|(id, values)| format!("`{id}` is {}", values.join(" or ")))
        .collect();
    Some(format!("Asked only when {}.", conditions.join(", and ")))
}

fn answers_table(decision: &interview::Decision) -> String {
    if decision.free_text {
        return format!(
            "Free text. {}\n",
            effects_summary(decision.effects.as_ref()),
        );
    }
    let mut out = String::from("| Answer | What it does to the policy |\n| :-- | :-- |\n");
    for option in &decision.options {
        out.push_str(&format!(
            "| `{}` | {} |\n",
            option.id,
            effects_summary(option.effects.as_ref()),
        ));
    }
    out
}

fn effects_summary(effects: Option<&interview::Effects>) -> String {
    let Some(effects) = effects else {
        return "Nothing on its own.".to_string();
    };
    let mut parts = Vec::new();
    push_ids(&mut parts, "enables", &effects.enable);
    push_ids(&mut parts, "switches off", &effects.disable);
    push_ids(&mut parts, "writes the template", &effects.templates);
    let keys: Vec<String> = effects.options.keys().cloned().collect();
    push_ids(&mut parts, "sets options on", &keys);
    let answered: Vec<String> = effects.options_from_answer.keys().cloned().collect();
    push_ids(&mut parts, "points at your own paths", &answered);
    if parts.is_empty() {
        return "Nothing on its own.".to_string();
    }
    format!("{}.", parts.join("; "))
}

fn push_ids(parts: &mut Vec<String>, verb: &str, ids: &[String]) {
    if ids.is_empty() {
        return;
    }
    let quoted: Vec<String> = ids.iter().map(|id| format!("`{id}`")).collect();
    parts.push(format!("{verb} {}", quoted.join(", ")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_marker_contents_change() {
        let mut blocks = BTreeMap::new();
        blocks.insert("x", "new".to_string());
        let input = "before\n<!-- sf:generated x -->\nold\n<!-- sf:end x -->\nafter\n";
        let (actual, _) = replace_blocks(input, &blocks).expect("marker replacement succeeds");
        assert_eq!(
            actual,
            "before\n<!-- sf:generated x -->\nnew\n<!-- sf:end x -->\nafter\n"
        );
    }

    #[test]
    fn unknown_and_unclosed_blocks_fail() {
        let blocks = BTreeMap::new();
        assert!(
            replace_blocks("<!-- sf:generated nope -->\n<!-- sf:end nope -->\n", &blocks).is_err()
        );
        let mut blocks = BTreeMap::new();
        blocks.insert("x", "new".to_string());
        assert!(replace_blocks("<!-- sf:generated x -->\n", &blocks).is_err());
    }

    /// The page that documents this mechanism has to be able to show a
    /// marker. Before fences were skipped it could only show an escaped one,
    /// which is a page teaching a form that does not work.
    #[test]
    fn a_marker_inside_a_fence_is_documentation() {
        let blocks = BTreeMap::new();
        let input = "```markdown\n<!-- sf:generated nope -->\n<!-- sf:end nope -->\n```\n";
        let (actual, found) = replace_blocks(input, &blocks).expect("a fenced marker is left be");
        assert_eq!(actual, input);
        assert!(found.is_empty());
    }

    #[test]
    fn every_block_renders_something() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalog = Catalog::builtin().expect("the embedded catalog loads");
        let rendered = blocks(root, &catalog).expect("every block renders");
        for name in BLOCKS {
            let body = rendered.get(name).unwrap_or_else(|| panic!("{name} is rendered"));
            assert!(!body.trim().is_empty(), "{name} rendered nothing");
        }
    }

    /// Two renders of one commit have to agree, or `--check` is a coin toss
    /// rather than a check.
    #[test]
    fn rendering_is_deterministic() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalog = Catalog::builtin().expect("the embedded catalog loads");
        let first = blocks(root, &catalog).expect("the first render succeeds");
        let second = blocks(root, &catalog).expect("the second render succeeds");
        assert_eq!(first, second);
    }
}
