use std::collections::BTreeMap;

use serde::Deserialize;

use super::scenario::Scenario;

pub(super) const PROCESS_EXPECTATIONS: &str = include_str!("expectations/process.toml");
pub(super) const WASM_EXPECTATIONS: &str = include_str!("expectations/wasm.toml");

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ExpectationFile {
    pub(super) rows: BTreeMap<String, Expectation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(super) enum Expectation {
    Pass,
    NotSupported {
        kind: String,
        reason: String,
        #[serde(default)]
        issue: Option<String>,
        #[serde(default)]
        temporary: bool,
        #[serde(default)]
        feature: Option<String>,
    },
}

impl Expectation {
    pub(super) fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }
}

pub(super) fn parse(name: &str, text: &str) -> ExpectationFile {
    toml::from_str(text).unwrap_or_else(|error| panic!("{name} expectations: {error}"))
}

pub(super) fn validate(scenarios: &[Scenario], file: &ExpectationFile) -> Vec<String> {
    let mut problems = Vec::new();
    for scenario in scenarios {
        match file.rows.get(scenario.id) {
            None => problems.push(format!("{}: missing status", scenario.id)),
            Some(Expectation::NotSupported {
                kind,
                reason,
                temporary,
                feature,
                ..
            }) => {
                if kind.trim().is_empty() {
                    problems.push(format!("{}: empty not-supported kind", scenario.id));
                }
                if reason.trim().is_empty() {
                    problems.push(format!("{}: empty not-supported reason", scenario.id));
                }
                if *temporary && feature.as_deref().is_none_or(str::is_empty) {
                    problems.push(format!(
                        "{}: temporary not-supported needs a feature",
                        scenario.id
                    ));
                }
            }
            Some(Expectation::Pass) => {}
        }
    }
    let known = scenarios
        .iter()
        .map(|scenario| scenario.id)
        .collect::<std::collections::BTreeSet<_>>();
    for id in file.rows.keys() {
        if !known.contains(id.as_str()) {
            problems.push(format!("{id}: status has no scenario"));
        }
    }
    problems
}
