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

/// Where Claude Code looks for personal skills.
pub fn default_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set; pass --dir")?;
    Ok(PathBuf::from(home).join(".claude/skills"))
}

pub fn install(dir: &Path) -> Result<Vec<String>> {
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
    Ok(written)
}
