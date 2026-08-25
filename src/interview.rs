//! The interview: a decision tree walked with a human, and the deterministic
//! mapping from their answers to policy.
//!
//! The split matters. Conducting the conversation is a judgement call and
//! belongs to whoever is running it. Turning "we use hexagonal architecture"
//! into a set of rules and globs is not, and it lives here as data — because
//! two people interviewing the same team must end up with the same policy, or
//! the whole exercise is just each agent's taste with extra steps.

use crate::catalog::Rule;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Effects {
    #[serde(default)]
    pub enable: Vec<String>,
    #[serde(default)]
    pub disable: Vec<String>,
    /// Option overrides, per rule id.
    #[serde(default)]
    pub options: BTreeMap<String, serde_yaml::Value>,
    /// Rule templates to instantiate into `.software-factory/rules/`.
    #[serde(default)]
    pub templates: Vec<String>,
    /// Rule id -> option name that takes this decision's free-text answer.
    #[serde(default)]
    pub options_from_answer: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecisionOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Effects>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Decision {
    pub id: String,
    pub question: String,
    /// Why this decision produces rules. The interviewer reads it aloud when
    /// asked "does this matter?", which is the most common follow-up.
    pub why: String,
    /// This decision is only asked when the named decisions took these values.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub depends_on: BTreeMap<String, Vec<String>>,
    /// Option id -> globs whose presence answers this without asking. Facts
    /// are the interviewer's job; only decisions go to the human.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub detect: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<DecisionOption>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub free_text: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    /// Effects for a free-text decision, which has no options to carry them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Effects>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Interview {
    pub version: u32,
    pub decisions: Vec<Decision>,
}

const DECISIONS: &str = include_str!("../interview/decisions.yaml");

const TEMPLATES: &[(&str, &str)] = &[
    ("schemas-live-with-their-handler", include_str!("../templates/schemas-live-with-their-handler.yaml")),
    ("no-fetch-inside-an-effect", include_str!("../templates/no-fetch-inside-an-effect.yaml")),
    ("global-state-lives-in-one-place", include_str!("../templates/global-state-lives-in-one-place.yaml")),
    ("client-never-imports-the-data-layer", include_str!("../templates/client-never-imports-the-data-layer.yaml")),
];

/// Rule ids the templates can produce. Documenting one is legitimate even
/// though it is not in the catalog until an interview instantiates it.
pub fn template_rule_ids() -> Vec<String> {
    TEMPLATES
        .iter()
        .filter_map(|(_, body)| {
            body.lines().find_map(|line| line.strip_prefix("id: ").map(str::to_string))
        })
        .collect()
}

impl Interview {
    pub fn load() -> Result<Interview> {
        let interview: Interview =
            serde_yaml::from_str(DECISIONS).context("the built-in decision tree is malformed")?;
        anyhow::ensure!(interview.version == 1, "unsupported interview version");
        Ok(interview)
    }

    pub fn get(&self, id: &str) -> Option<&Decision> {
        self.decisions.iter().find(|d| d.id == id)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Answers {
    pub version: u32,
    /// Decision id -> chosen option id, or free text.
    pub answers: BTreeMap<String, String>,
}

impl Answers {
    pub fn load(path: &Path) -> Result<Answers> {
        let body = std::fs::read_to_string(path)
            .with_context(|| format!("no answers at {}", path.display()))?;
        let answers: Answers =
            serde_yaml::from_str(&body).with_context(|| format!("{} is malformed", path.display()))?;
        anyhow::ensure!(answers.version == 1, "unsupported answers version");
        Ok(answers)
    }
}

/// What the answers add up to, before any of it touches a file.
#[derive(Debug, Default)]
pub struct Plan {
    pub enable: BTreeSet<String>,
    pub disable: BTreeSet<String>,
    pub options: BTreeMap<String, serde_yaml::Value>,
    pub templates: Vec<String>,
    /// `${...}` substitutions available to templates.
    pub vars: BTreeMap<String, String>,
}

/// Fold the answers into a plan. Unknown decisions and unknown options are
/// errors, not warnings: a typo in an answers file must not quietly produce a
/// weaker policy than the one that was agreed.
pub fn plan(interview: &Interview, answers: &Answers) -> Result<Plan> {
    let mut plan = Plan::default();
    for (decision_id, answer) in &answers.answers {
        let decision = interview
            .get(decision_id)
            .with_context(|| format!("no decision {decision_id:?} in the interview"))?;
        let effects = if decision.free_text {
            expand_vars(&mut plan, decision_id, answer);
            let mut effects = decision.effects.clone().unwrap_or_default();
            if split(answer).is_empty() {
                // An empty free-text answer means "there are none of these".
                // Enabling the rule anyway leaves it switched on and pointed
                // at nothing, which L5.NO_INERT_RULE would then reject —
                // correctly, but with the person wondering what they did wrong.
                effects.disable.append(&mut effects.enable);
                effects.templates.clear();
                effects.options_from_answer.clear();
            }
            effects
        } else {
            let option = decision
                .options
                .iter()
                .find(|o| o.id == *answer)
                .with_context(|| {
                    format!(
                        "{decision_id}: {answer:?} is not one of {}",
                        decision.options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>().join(", ")
                    )
                })?;
            plan.vars.insert(decision_id.clone(), option.id.clone());
            option.effects.clone().unwrap_or_default()
        };
        apply(&mut plan, decision_id, answer, &effects)?;
    }
    // An explicit disable beats an enable inherited from another answer: the
    // person said "there is no database", and no other decision overrides that.
    for id in &plan.disable {
        plan.enable.remove(id);
    }
    Ok(plan)
}

fn apply(plan: &mut Plan, decision_id: &str, answer: &str, effects: &Effects) -> Result<()> {
    plan.enable.extend(effects.enable.iter().cloned());
    plan.disable.extend(effects.disable.iter().cloned());
    for (rule, value) in &effects.options {
        let merged = match plan.options.get(rule) {
            Some(existing) => crate::policy::merge(existing, value),
            None => value.clone(),
        };
        plan.options.insert(rule.clone(), merged);
    }
    for template in &effects.templates {
        if !plan.templates.contains(template) {
            plan.templates.push(template.clone());
        }
    }
    for (rule, option_name) in &effects.options_from_answer {
        let items = split(answer);
        if items.is_empty() {
            continue;
        }
        let mut mapping = serde_yaml::Mapping::new();
        mapping.insert(
            serde_yaml::Value::String(option_name.clone()),
            serde_yaml::to_value(&items)?,
        );
        let value = serde_yaml::Value::Mapping(mapping);
        let merged = match plan.options.get(rule) {
            Some(existing) => crate::policy::merge(existing, &value),
            None => value,
        };
        plan.options.insert(rule.clone(), merged);
        plan.enable.insert(rule.clone());
    }
    let _ = decision_id;
    Ok(())
}

fn split(answer: &str) -> Vec<String> {
    answer
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Derived forms of a free-text answer, so templates never have to parse it.
fn expand_vars(plan: &mut Plan, id: &str, answer: &str) {
    let items = split(answer);
    plan.vars.insert(id.to_string(), answer.trim().to_string());
    if let Some(first) = items.first() {
        plan.vars.insert(format!("{id}_first"), first.clone());
    }
    plan.vars.insert(format!("{id}_list"), yaml_list(&items));
    plan.vars.insert(
        format!("{id}_globs"),
        yaml_list(&items.iter().map(|item| format!("{}/**", item.trim_end_matches('/'))).collect::<Vec<_>>()),
    );
    plan.vars.insert(
        format!("{id}_pattern"),
        items.iter().map(|item| regex_escape(item)).collect::<Vec<_>>().join("|"),
    );
}

fn yaml_list(items: &[String]) -> String {
    format!("[{}]", items.iter().map(|i| format!("\"{i}\"")).collect::<Vec<_>>().join(", "))
}

fn regex_escape(item: &str) -> String {
    // Doubled, because the result is written into a YAML scalar that is then
    // parsed as a tree-sitter query string.
    item.chars()
        .map(|c| match c {
            '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                format!("\\\\{c}")
            }
            _ => c.to_string(),
        })
        .collect()
}

pub struct Instantiated {
    pub rule: Rule,
    pub body: String,
    pub fixture: Vec<(String, String)>,
}

/// Fill a template's `${...}` holes and split off the fixture it carries.
pub fn instantiate(name: &str, vars: &BTreeMap<String, String>) -> Result<Instantiated> {
    let (_, source) = TEMPLATES
        .iter()
        .find(|(id, _)| *id == name)
        .with_context(|| format!("no rule template named {name:?}"))?;
    // `@@name@@` rather than `${name}`: fixtures contain real source in four
    // languages, and `${...}` is a template literal in two of them.
    let mut filled = source.to_string();
    for (key, value) in vars {
        filled = filled.replace(&format!("@@{key}@@"), value);
    }
    if let Some(start) = filled.find("@@") {
        let end = filled[start + 2..].find("@@").map(|e| start + e + 4).unwrap_or(filled.len());
        bail!(
            "template {name} still needs {} — the interview did not answer it",
            &filled[start..end]
        );
    }

    let mut value: serde_yaml::Value = serde_yaml::from_str(&filled)
        .with_context(|| format!("template {name} is malformed once filled in"))?;
    let fixture = value
        .as_mapping_mut()
        .and_then(|m| m.remove(serde_yaml::Value::String("fixture".into())))
        .unwrap_or(serde_yaml::Value::Null);
    let fixture: Vec<FixtureFile> = serde_yaml::from_value(fixture).unwrap_or_default();

    let body = serde_yaml::to_string(&value)?;
    let rule: Rule = serde_yaml::from_value(value)
        .with_context(|| format!("template {name} does not describe a valid rule"))?;
    rule.validate().with_context(|| format!("template {name} is not runnable once filled in"))?;
    Ok(Instantiated {
        rule,
        body,
        fixture: fixture.into_iter().map(|f| (f.path, f.body)).collect(),
    })
}

#[derive(Deserialize, Default)]
struct FixtureFile {
    path: String,
    body: String,
}

#[cfg(test)]
mod completeness {
    use super::TEMPLATES;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    fn templates_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("templates")
    }

    fn on_disk() -> BTreeSet<String> {
        std::fs::read_dir(templates_dir())
            .expect("templates/ exists")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("yaml")
            })
            .map(|e| {
                e.path()
                    .file_stem()
                    .expect("yaml file has a stem")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    /// `TEMPLATES` is the set of parameterised rules an interview can
    /// instantiate. A `templates/*.yaml` with no entry here is unreachable
    /// from `sf init`. Both directions, offenders named.
    #[test]
    fn templates_matches_templates_dir() {
        let registered: BTreeSet<String> = TEMPLATES.iter().map(|(name, _)| name.to_string()).collect();
        let disk = on_disk();

        let unregistered: Vec<_> = disk.difference(&registered).collect();
        let missing: Vec<_> = registered.difference(&disk).collect();

        assert!(
            unregistered.is_empty() && missing.is_empty(),
            "interview::TEMPLATES is out of sync with templates/:\n\
             files on disk with no TEMPLATES entry: {unregistered:?}\n\
             TEMPLATES entries with no file on disk: {missing:?}"
        );
    }
}
