//! L2 — a check this tool cannot express, run where it can still be seen.
//!
//! Some drift is only decidable by regenerating the artifact: export the API
//! schema, run the generator, diff the result. No glob or query says that, and
//! pretending otherwise would mean shipping a worse version of a check a team
//! already has.
//!
//! So a rule may name a command. What it buys over a plain CI step is
//! everything around it: a written reason printed at the point of failure, a
//! mutation fixture proving it still fails when it should, a place in the same
//! report, and a policy that cannot be quietly loosened.
//!
//! Commands are refused unless explicitly allowed. A policy file is data that
//! travels with a clone, and running whatever it says on `sf check` would make
//! cloning a repository dangerous.

use super::Ctx;
use crate::catalog::Rule;
use crate::finding::{Finding, Severity};
use crate::policy::Options;
use crate::scan;
use anyhow::Result;
use std::process::Command;

pub fn run(rule: &Rule, opts: &Options, ctx: &Ctx) -> Result<Vec<Finding>> {
    let Some(command) = opts.run.as_deref() else {
        return Ok(Vec::new());
    };
    // Scoped away: nothing this command is about is present here.
    if !opts.scope.is_empty() {
        let scope = scan::globs(&opts.scope)?;
        if !ctx.files.iter().any(|f| scope.is_match(&f.rel)) {
            return Ok(Vec::new());
        }
    }
    if !ctx.allow_commands {
        return Ok(vec![
            Finding::new(
                &rule.id,
                Severity::Medium,
                crate::policy::POLICY_PATH,
                format!("{}:not-run", rule.id),
                "this rule runs a command, and commands are not enabled",
            )
            .expected("sf check --allow-commands, or SF_ALLOW_COMMANDS=1")
            .actual(command.to_string()),
        ]);
    }

    let output = Command::new("sh").arg("-c").arg(command).current_dir(ctx.root).output()?;
    if output.status.success() {
        return Ok(Vec::new());
    }
    let mut detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        detail = String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    Ok(vec![
        Finding::new(
            &rule.id,
            rule.severity,
            command.to_string(),
            format!("{}:failed", rule.id),
            format!(
                "`{command}` exited {}",
                output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "on a signal".into())
            ),
        )
        .expected("exit 0")
        .actual(tail(&detail, 20)),
    ])
}

/// The last lines only. A failing generator can print a great deal, and the
/// end of it is where the reason usually is.
fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}
