//! Repo-relative file walking and glob matching.

use crate::policy::{ALWAYS_SKIP, Policy};
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};

/// Build a matcher. `**/name` is also registered as bare `name`, so a pattern
/// meant as "anywhere" still matches a file sitting at the repository root.
pub fn globs(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
        if let Some(rest) = pattern.strip_prefix("**/") {
            builder.add(Glob::new(rest)?);
        }
    }
    Ok(builder.build()?)
}


#[derive(Debug, Clone)]
pub struct SourceFile {
    pub rel: String,
    pub abs: PathBuf,
}

/// Every tracked-ish file in the repository, minus universal noise and the
/// repo's own excludes. Deterministic order: findings must be stable between
/// runs or the ratchet keys churn.
pub fn walk(root: &Path, policy: &Policy) -> Result<Vec<SourceFile>> {
    let excludes = globs(&policy.project.exclude)?;
    let mut files = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    collect(root, root, false, &excludes, &mut files, &mut seen)?;
    for extra in &policy.project.roots {
        collect(root, &root.join(extra), true, &excludes, &mut files, &mut seen)?;
    }
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(files)
}

fn collect(
    root: &Path,
    start: &Path,
    follow_links: bool,
    excludes: &globset::GlobSet,
    files: &mut Vec<SourceFile>,
    seen: &mut std::collections::BTreeSet<String>,
) -> Result<()> {
    if !start.exists() {
        return Ok(());
    }
    // `.gitignore` is respected, and that is not a convenience. Build output,
    // run artifacts and caches are ignored precisely because they are not the
    // repository's work, and scanning them produces findings nobody can act on
    // — worse, a seeded ratchet full of them, committed.
    let mut builder = ignore::WalkBuilder::new(start);
    builder
        .follow_links(follow_links)
        .git_ignore(true)
        .git_global(false)
        .require_git(false)
        .hidden(false)
        .sort_by_file_name(|a, b| a.cmp(b))
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !ALWAYS_SKIP.contains(&name.as_ref())
        });
    for entry in builder.build() {
        // A dangling symlink, or a directory this user cannot read, is not a
        // reason to abandon the scan. Following links makes both ordinary.
        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Ok(rel_path) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel_path.to_string_lossy().replace('\\', "/");
        // Mutation fixtures are deliberately broken repositories. Walking them
        // here would make every fixture a finding in its host.
        if rel.starts_with(".software-factory/mutations/") {
            continue;
        }
        if excludes.is_match(&rel) || !seen.insert(rel.clone()) {
            continue;
        }
        files.push(SourceFile { rel, abs: entry.path().to_path_buf() });
    }
    Ok(())
}

/// Files inside `scope` (empty scope means "everything") and outside `exclude`.
pub fn select<'a>(
    files: &'a [SourceFile],
    scope: &[String],
    exclude: &[String],
) -> Result<Vec<&'a SourceFile>> {
    let scoped = if scope.is_empty() { None } else { Some(globs(scope)?) };
    let excluded = if exclude.is_empty() { None } else { Some(globs(exclude)?) };
    Ok(files
        .iter()
        .filter(|f| scoped.as_ref().is_none_or(|g| g.is_match(&f.rel)))
        .filter(|f| !excluded.as_ref().is_some_and(|g| g.is_match(&f.rel)))
        .collect())
}
