//! The agent skills, shipped inside the binary.
//!
//! They were previously copied out of a checkout, which meant `cargo install
//! --git` — the one-line install — could not reach them, and it meant a skill
//! could drift out of step with the binary it drives. A skill telling you to
//! run a subcommand your `sf` predates is worse than no skill.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub const SKILLS: &[(&str, &str)] = &[
    ("factory-init", include_str!("../skills/factory-init/SKILL.md")),
    ("factory-author", include_str!("../skills/factory-author/SKILL.md")),
    ("factory-evidence", include_str!("../skills/factory-evidence/SKILL.md")),
    ("factory-triage", include_str!("../skills/factory-triage/SKILL.md")),
];

/// A skill removed from a shipped set must not remain discoverable after an
/// upgrade. Only its managed entrypoint is removed; a directory carrying a
/// user's other files is left intact.
const RETIRED_SKILLS: &[&str] = &["factory-harness"];

pub struct Installed {
    pub written: Vec<String>,
    pub removed: Vec<String>,
}

/// Where Claude Code looks for skills, in both scopes.
pub fn project_dir() -> PathBuf {
    PathBuf::from(".claude/skills")
}

pub fn user_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set; pass --dir")?;
    Ok(PathBuf::from(home).join(".claude/skills"))
}

/// Ask where to install. There is deliberately no default: these skills are
/// about *this* repository's factory, and quietly writing them into every
/// project on the machine is a decision nobody made. When nothing can be
/// asked — a script, CI, a pipe — say so rather than guessing.
pub fn choose_dir(root: &Path) -> Result<PathBuf> {
    use std::io::{IsTerminal, Write, stdin, stdout};
    if !stdin().is_terminal() {
        anyhow::bail!(
            "nothing to ask on: pass --dir, or --project for {}/.claude/skills, \
             or --user for ~/.claude/skills",
            root.display()
        );
    }
    let user = user_dir()?;
    println!("Where should the skills go?\n");
    println!("  1  {}/.claude/skills   (this repository only)", root.display());
    println!("  2  {}          (every project on this machine)", user.display());
    print!("\n[1] ");
    stdout().flush()?;
    let mut answer = String::new();
    stdin().read_line(&mut answer)?;
    match answer.trim() {
        "" | "1" => Ok(root.join(project_dir())),
        "2" => Ok(user),
        other => Ok(PathBuf::from(other)),
    }
}

pub fn install(dir: &Path) -> Result<Installed> {
    let mut written = Vec::new();
    for (name, body) in SKILLS {
        let target = dir.join(name);
        std::fs::create_dir_all(&target)
            .with_context(|| format!("could not create {}", target.display()))?;
        let path = target.join("SKILL.md");
        std::fs::write(&path, body)
            .with_context(|| format!("could not write {}", path.display()))?;
        written.push(path.display().to_string());
    }
    let mut removed = Vec::new();
    for name in RETIRED_SKILLS {
        let target = dir.join(name);
        let path = target.join("SKILL.md");
        if path.is_file() {
            std::fs::remove_file(&path)
                .with_context(|| format!("could not remove retired skill {}", path.display()))?;
            removed.push(path.display().to_string());
            // A retired skill normally owns its directory, but never remove
            // a directory that acquired a user's supplementary files.
            let _ = std::fs::remove_dir(target);
        }
    }
    Ok(Installed { written, removed })
}

#[cfg(test)]
mod completeness {
    use super::{SKILLS, install};
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    fn skills_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("skills")
    }

    fn on_disk() -> BTreeSet<String> {
        std::fs::read_dir(skills_dir())
            .expect("skills/ exists")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && e.path().join("SKILL.md").is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()
    }

    /// The bug AGENTS.md names: a new `skills/<name>/SKILL.md` with no
    /// matching entry here ships three of four, silently. Both directions,
    /// offenders named.
    #[test]
    fn skills_matches_skills_dir() {
        let registered: BTreeSet<String> = SKILLS.iter().map(|(name, _)| name.to_string()).collect();
        let disk = on_disk();

        let unregistered: Vec<_> = disk.difference(&registered).collect();
        let missing: Vec<_> = registered.difference(&disk).collect();

        assert!(
            unregistered.is_empty() && missing.is_empty(),
            "skills::SKILLS is out of sync with skills/:\n\
             directories on disk with no SKILLS entry: {unregistered:?}\n\
             SKILLS entries with no directory on disk: {missing:?}"
        );
    }

    #[test]
    fn an_upgrade_removes_a_retired_managed_skill_but_keeps_user_files() {
        let root = std::env::temp_dir().join(format!(
            "sf-skills-upgrade-{}",
            std::process::id()
        ));
        let retired = root.join("factory-harness");
        std::fs::create_dir_all(&retired).expect("retired skill directory");
        std::fs::write(retired.join("SKILL.md"), "old skill").expect("old skill");
        std::fs::write(retired.join("notes.md"), "keep this").expect("user note");

        let installed = install(&root).expect("skills install");

        assert_eq!(installed.removed, vec![retired.join("SKILL.md").display().to_string()]);
        assert!(!retired.join("SKILL.md").exists(), "the stale entrypoint is gone");
        assert!(retired.join("notes.md").is_file(), "user files survive the upgrade");
        std::fs::remove_dir_all(root).expect("scratch cleanup");
    }
}
