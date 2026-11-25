use std::collections::BTreeMap;
use std::fmt;

use super::harness::Runtime;

#[derive(Debug, Default)]
pub(super) struct RuntimeReport {
    passed: usize,
    not_supported: BTreeMap<String, usize>,
    failures: Vec<String>,
}

impl RuntimeReport {
    pub(super) fn record_pass(&mut self) {
        self.passed += 1;
    }

    pub(super) fn record_not_supported(&mut self, kind: &str) {
        *self.not_supported.entry(kind.to_string()).or_default() += 1;
    }

    pub(super) fn record_failure(&mut self, failure: String) {
        self.failures.push(failure);
    }
}

#[derive(Debug, Default)]
pub(super) struct Report {
    runtimes: BTreeMap<&'static str, RuntimeReport>,
    not_built: BTreeMap<&'static str, &'static str>,
}

impl Report {
    pub(super) fn runtime(&mut self, runtime: Runtime) -> &mut RuntimeReport {
        self.runtimes.entry(runtime.label()).or_default()
    }

    pub(super) fn record_not_built(&mut self, runtime: Runtime, reason: &'static str) {
        self.not_built.insert(runtime.label(), reason);
    }

    pub(super) fn has_failures(&self) -> bool {
        self.runtimes
            .values()
            .any(|report| !report.failures.is_empty())
    }

    pub(super) fn failure_messages(&self) -> Vec<String> {
        let mut failures = Vec::new();
        for (runtime, report) in &self.runtimes {
            for failure in &report.failures {
                failures.push(format!("{runtime}: {failure}"));
            }
        }
        failures
    }
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "conformance report")?;
        for (runtime, report) in &self.runtimes {
            let not_supported: usize = report.not_supported.values().sum();
            writeln!(
                formatter,
                "{runtime}: pass {} / not-supported {not_supported}",
                report.passed
            )?;
            if !report.not_supported.is_empty() {
                writeln!(formatter, "{runtime} not-supported by kind:")?;
                for (kind, count) in &report.not_supported {
                    writeln!(formatter, "  {kind}: {count}")?;
                }
            }
            if !report.failures.is_empty() {
                writeln!(formatter, "{runtime} failures: {}", report.failures.len())?;
            }
        }
        for (runtime, reason) in &self.not_built {
            writeln!(formatter, "{runtime}: not built ({reason})")?;
        }
        Ok(())
    }
}
