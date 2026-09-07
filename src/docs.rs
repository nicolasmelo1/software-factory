//! Deterministic, marker-scoped documentation generation.

use crate::catalog::{Catalog, CheckKind, Rule};
use crate::init;
use crate::policy::Policy;
use crate::ratchet::Ratchet;
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const BLOCKS: &[&str] = &["rules-summary", "command-surface", "language-coverage"];

pub struct RenderedPage {
    pub path: PathBuf,
    pub content: String,
}

pub fn render(root: &Path, catalog: &Catalog) -> Result<Vec<RenderedPage>> {
    let policy = Policy::load(root)?;
    let ratchet = Ratchet::load(root)?;
    let mut generated = BTreeMap::new();
    generated.insert("rules-summary", rules_summary(catalog, &policy, &ratchet));
    generated.insert("command-surface", command_surface());
    generated.insert("language-coverage", language_coverage(catalog, &policy));

    let mut pages = markdown_pages(root)?;
    let mut found = BTreeSet::new();
    let mut rendered = Vec::new();
    for path in pages.drain(..) {
        let original = std::fs::read_to_string(root.join(&path))?;
        let (content, names) = replace_blocks(&original, &generated)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        found.extend(names);
        rendered.push(RenderedPage { path, content });
    }
    let missing: Vec<_> = BLOCKS
        .iter()
        .filter(|name| !found.contains(**name))
        .collect();
    if !missing.is_empty() {
        bail!(
            "generated block(s) have no placement: {}",
            missing.iter().map(|s| **s).collect::<Vec<_>>().join(", ")
        );
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
    let pages = render(root, catalog)?;
    let mut written = Vec::new();
    for page in pages {
        let path = root.join(&page.path);
        let current = std::fs::read_to_string(&path)?;
        if current != page.content {
            std::fs::write(&path, page.content)?;
            written.push(page.path);
        }
    }
    Ok(written)
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

fn replace_blocks(
    input: &str,
    generated: &BTreeMap<&str, String>,
) -> Result<(String, BTreeSet<String>)> {
    let mut out = String::with_capacity(input.len());
    let mut found = BTreeSet::new();
    let mut lines = input.split_inclusive('\n');
    while let Some(line) = lines.next() {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let Some(name) = trimmed
            .trim()
            .strip_prefix("<!-- sf:generated ")
            .and_then(|s| s.strip_suffix(" -->"))
        else {
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
        let end = format!("<!-- sf:end {name} -->");
        let mut closed = false;
        for inner in lines.by_ref() {
            if inner.trim_end_matches(['\r', '\n']).trim() == end {
                out.push_str(&format!("{}\n", generated[name]));
                out.push_str(inner);
                closed = true;
                break;
            }
        }
        if !closed {
            bail!("generated block `{name}` has no end marker");
        }
    }
    Ok((out, found))
}

fn rules_summary(catalog: &Catalog, policy: &Policy, ratchet: &Ratchet) -> String {
    let shipped = catalog.rules.len();
    let enabled_ids: BTreeSet<_> = policy
        .instances()
        .into_iter()
        .map(|(_, id)| crate::policy::base_rule_id(&id).to_string())
        .collect();
    let enabled = enabled_ids.len();
    let frozen: usize = ratchet.rules.values().map(|r| r.allow.len()).sum();
    let reviews: Vec<_> = ratchet
        .rules
        .iter()
        .map(|(id, r)| format!("- `{id}`: {}", r.review_by))
        .collect();
    format!(
        "**Rules:** {shipped} shipped; {enabled} enabled; {} switched off; {enabled} proven to fire; {frozen} violations frozen.\n\n{}",
        shipped.saturating_sub(enabled),
        if reviews.is_empty() {
            "No frozen violations.".to_string()
        } else {
            format!("**Frozen review dates**\n{}", reviews.join("\n"))
        }
    )
}

fn command_surface() -> String {
    let mut out = String::from("| Command | Long flags |\n| :-- | :-- |\n");
    for (command, flags) in crate::accepted_commands() {
        out.push_str(&format!(
            "| `sf {command}` | {} |\n",
            flags.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    out
}

fn language_coverage(catalog: &Catalog, policy: &Policy) -> String {
    let languages: BTreeSet<_> = policy.project.languages.iter().cloned().collect();
    let mut out = String::from("| Rule | Languages with a query |\n| :-- | :-- |\n");
    for rule in catalog.rules.values() {
        let covered = rule_languages(rule, &languages);
        out.push_str(&format!(
            "| `{}` | {} |\n",
            rule.id,
            if covered.is_empty() {
                "—".to_string()
            } else {
                covered.join(", ")
            }
        ));
    }
    out
}

fn rule_languages(rule: &Rule, declared: &BTreeSet<String>) -> Vec<String> {
    let names: BTreeSet<String> = match &rule.check {
        CheckKind::Shape { languages } => languages.keys().cloned().collect(),
        CheckKind::Forwarder { languages } => languages.keys().cloned().collect(),
        CheckKind::Nested { languages } => languages.keys().cloned().collect(),
        CheckKind::Complexity => declared.clone(),
        _ => BTreeSet::new(),
    };
    names
        .into_iter()
        .filter(|name| declared.contains(name))
        .collect()
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
            replace_blocks(
                "<!-- sf:generated nope -->\n<!-- sf:end nope -->\n",
                &blocks
            )
            .is_err()
        );
        let mut blocks = BTreeMap::new();
        blocks.insert("x", "new".to_string());
        assert!(replace_blocks("<!-- sf:generated x -->\n", &blocks).is_err());
    }
}
