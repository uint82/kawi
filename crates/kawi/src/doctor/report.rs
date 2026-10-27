use serde::Serialize;
use std::fmt::Write as _;
use std::path::Path;

pub(crate) const REPORT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Level {
    Ok,
    Info,
    Warn,
    Error,
}

impl Level {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Check {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) level: Level,
    pub(crate) details: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hint: Option<String>,
}

impl Check {
    pub(crate) fn new(
        id: &'static str,
        title: &'static str,
        level: Level,
        details: Vec<String>,
    ) -> Self {
        Self {
            id,
            title,
            level,
            details,
            hint: None,
        }
    }

    pub(crate) fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Summary {
    pub(crate) status: Level,
    pub(crate) ok: usize,
    pub(crate) info: usize,
    pub(crate) warn: usize,
    pub(crate) error: usize,
}

impl Summary {
    fn from_checks(checks: &[Check]) -> Self {
        let mut summary = Self {
            status: Level::Ok,
            ok: 0,
            info: 0,
            warn: 0,
            error: 0,
        };
        for check in checks {
            summary.status = summary.status.max(check.level);
            match check.level {
                Level::Ok => summary.ok += 1,
                Level::Info => summary.info += 1,
                Level::Warn => summary.warn += 1,
                Level::Error => summary.error += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub(crate) schema: u32,
    pub(crate) kawi_version: &'static str,
    pub(crate) os: &'static str,
    pub(crate) arch: &'static str,
    pub(crate) target: String,
    pub(crate) workspace: String,
    pub(crate) checks: Vec<Check>,
    pub(crate) summary: Summary,
}

impl Report {
    pub(crate) fn new(target: &Path, workspace: &Path, checks: Vec<Check>) -> Self {
        let summary = Summary::from_checks(&checks);
        Self {
            schema: REPORT_SCHEMA,
            kawi_version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            target: target.display().to_string(),
            workspace: workspace.display().to_string(),
            checks,
            summary,
        }
    }

    pub(crate) fn exit_code(&self, strict: bool) -> i32 {
        if self.summary.error > 0 || (strict && self.summary.warn > 0) {
            1
        } else {
            0
        }
    }

    pub(crate) fn to_text(&self) -> String {
        let mut text = String::new();
        push_header(self, &mut text);
        for check in &self.checks {
            push_check(check, &mut text);
        }
        let _ = writeln!(
            text,
            "Summary: ok {}, info {}, warnings {}, errors {}",
            self.summary.ok, self.summary.info, self.summary.warn, self.summary.error
        );
        text
    }
}

fn push_header(report: &Report, text: &mut String) {
    let _ = writeln!(text, "Kawi doctor {}", report.kawi_version);
    let _ = writeln!(text, "platform {} {}", report.os, report.arch);
    let _ = writeln!(text, "target {}", report.target);
    let _ = writeln!(text, "workspace {}", report.workspace);
    let _ = writeln!(text);
}

fn push_check(check: &Check, text: &mut String) {
    let _ = writeln!(text, "[{}] {}", check.level.label(), check.title);
    for detail in &check.details {
        let _ = writeln!(text, "  {detail}");
    }
    if let Some(hint) = check.hint.as_deref() {
        let _ = writeln!(text, "  hint: {hint}");
    }
    let _ = writeln!(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(levels: &[Level]) -> Report {
        let checks = levels
            .iter()
            .enumerate()
            .map(|(index, level)| {
                Check::new("check", "Check", *level, vec![format!("detail {index}")])
            })
            .collect();
        Report::new(Path::new("/project"), Path::new("/project"), checks)
    }

    #[test]
    fn summary_counts_levels_and_uses_the_worst_status() {
        let report = report(&[Level::Ok, Level::Info, Level::Warn]);
        assert_eq!(report.summary.status, Level::Warn);
        assert_eq!(report.summary.ok, 1);
        assert_eq!(report.summary.info, 1);
        assert_eq!(report.summary.warn, 1);
        assert_eq!(report.summary.error, 0);
    }

    #[test]
    fn warnings_only_fail_in_strict_mode() {
        let warning_report = report(&[Level::Warn]);
        assert_eq!(warning_report.exit_code(false), 0);
        assert_eq!(warning_report.exit_code(true), 1);
        assert_eq!(report(&[Level::Error]).exit_code(false), 1);
    }

    #[test]
    fn text_output_includes_details_hints_and_summary() {
        let mut report = report(&[Level::Warn]);
        report.checks[0].details.push("more detail".to_string());
        report.checks[0].hint = Some("fix it".to_string());
        let text = report.to_text();
        assert!(text.contains("[warn] Check"));
        assert!(text.contains("  detail 0"));
        assert!(text.contains("  more detail"));
        assert!(text.contains("  hint: fix it"));
        assert!(text.contains("Summary: ok 0, info 0, warnings 1, errors 0"));
    }
}
