//! Typed findings. The set of rule ids is this tool's public contract:
//! renaming one is a breaking change for every repo that pinned it.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        };
        f.write_str(s)
    }
}

/// A single violation. `key` is the stable identity used by the ratchet:
/// two runs over the same unchanged violation must produce the same key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub location: String,
    pub key: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ratcheted: bool,
}

impl Finding {
    pub fn new(
        rule: &str,
        severity: Severity,
        location: impl Into<String>,
        key: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Finding {
            rule: rule.to_string(),
            severity,
            location: location.into(),
            key: key.into(),
            message: message.into(),
            expected: None,
            actual: None,
            ratcheted: false,
        }
    }

    pub fn expected(mut self, v: impl Into<String>) -> Self {
        self.expected = Some(v.into());
        self
    }

    pub fn actual(mut self, v: impl Into<String>) -> Self {
        self.actual = Some(v.into());
        self
    }
}

/// Exit codes are hierarchical so a caller can tell "the tool could not run"
/// apart from "the repo has violations". Highest condition wins.
pub const EXIT_OK: i32 = 0;
pub const EXIT_FINDINGS: i32 = 1;
pub const EXIT_CONFIG: i32 = 2;
pub const EXIT_BOOTSTRAP: i32 = 3;
