use crossterm::event::{KeyCode, KeyEvent};

use super::fs::RealFs;
use crate::app::App;
use crate::recovery::ApplyRecovery;

mod complete;
mod report;
mod rollback;

use complete::complete;
pub(in crate::app) use complete::complete_cleanups;
use report::RecoveryReport;
use rollback::recover;

pub(in crate::app) struct PendingRecovery {
    recovery: ApplyRecovery,
    shown: bool,
}

impl PendingRecovery {
    fn new(recovery: ApplyRecovery) -> Self {
        Self {
            recovery,
            shown: false,
        }
    }

    fn lines(&self, width: usize) -> Vec<String> {
        let text = format!(
            "Interrupted apply operation detected.\n\n{} filesystem operation(s) may be partially applied.\n\nr roll back reversible changes  k keep current files",
            self.recovery.journal.operations.len()
        );
        super::super::pager_lines(&text, width)
    }

    fn shown(&self) -> bool {
        self.shown
    }

    fn mark_shown(&mut self) {
        self.shown = true;
    }
}

#[derive(Clone, Copy)]
enum Choice {
    Rollback,
    Keep,
}

fn choice(key: KeyEvent) -> Option<Choice> {
    match key.code {
        KeyCode::Char('r' | 'R') => Some(Choice::Rollback),
        KeyCode::Char('k' | 'K') | KeyCode::Enter => Some(Choice::Keep),
        _ => None,
    }
}

impl App {
    pub(in crate::app) fn start_apply_recovery(&mut self, recovery: ApplyRecovery) {
        if recovery.journal.is_prepared() {
            self.apply_recovery = Some(PendingRecovery::new(recovery));
        } else {
            self.complete_apply_recovery(&recovery);
        }
    }

    fn complete_apply_recovery(&mut self, recovery: &ApplyRecovery) {
        let mut fs = RealFs::new(false);
        let report = complete(&recovery.journal, &mut fs);
        let mut message = report_message(&report);
        if report.is_incomplete() {
            message.push_str("; recovery journal kept");
        } else if let Some(error) = self.remove_apply_recovery_files() {
            message.push_str("; ");
            message.push_str(&error);
        }
        self.show_message(message);
    }

    pub(in crate::app) fn show_apply_recovery_prompt_if_ready(&mut self) -> bool {
        let Some(prompt) = self.apply_recovery.as_ref() else {
            return false;
        };
        if prompt.shown() || self.pager.is_some() {
            return true;
        }
        let lines = prompt.lines(self.screen_width);
        if let Some(prompt) = self.apply_recovery.as_mut() {
            prompt.mark_shown();
        }
        self.show_pager(lines);
        true
    }

    pub(in crate::app) fn apply_recovery_prompt_visible(&self) -> bool {
        self.apply_recovery
            .as_ref()
            .is_some_and(PendingRecovery::shown)
    }

    pub(in crate::app) fn handle_apply_recovery_prompt_key(&mut self, key: KeyEvent) -> bool {
        let Some(selection) = choice(key) else {
            return false;
        };
        let Some(prompt) = self.apply_recovery.take() else {
            return false;
        };
        self.pager = None;
        match selection {
            Choice::Rollback => self.rollback_apply_recovery(&prompt.recovery),
            Choice::Keep => self.keep_apply_recovery(),
        }
        self.show_recovery_prompt_if_ready();
        false
    }

    fn rollback_apply_recovery(&mut self, recovery: &ApplyRecovery) {
        let mut fs = RealFs::new(false);
        let report = recover(&recovery.journal, &recovery.promotions, &mut fs);
        let mut message = report_message(&report);
        if report.is_incomplete() {
            message.push_str("; recovery journal kept");
        } else if let Some(error) = self.remove_apply_recovery_files() {
            message.push_str("; ");
            message.push_str(&error);
        }
        self.show_message(message);
    }

    fn keep_apply_recovery(&mut self) {
        match self.remove_apply_recovery_files() {
            Some(error) => self.show_message(error),
            None => self.show_message("Apply recovery journal discarded".to_string()),
        }
    }

    fn remove_apply_recovery_files(&self) -> Option<String> {
        let manager = self.recovery.as_ref()?;
        manager
            .remove_apply_journal()
            .and_then(|()| manager.remove_apply_promotions())
            .err()
            .map(|error| format!("cannot remove apply recovery journal: {error}"))
    }
}

fn report_message(report: &RecoveryReport) -> String {
    let mut parts = Vec::new();
    if report.restored > 0 {
        parts.push(format!("restored {}", report.restored));
    }
    if report.removed > 0 {
        parts.push(format!("removed {}", report.removed));
    }
    if !report.irreversible.is_empty() {
        parts.push(format!(
            "{} delete(s) already finalized: {}",
            report.irreversible.len(),
            summarize(&report.irreversible)
        ));
    }
    if !report.skipped.is_empty() {
        parts.push(format!(
            "{} ambiguous path(s): {}",
            report.skipped.len(),
            summarize(&report.skipped)
        ));
    }
    if !report.errors.is_empty() {
        parts.push(format!(
            "{} error(s): {}",
            report.errors.len(),
            summarize(&report.errors)
        ));
    }
    if parts.is_empty() {
        "Apply recovery complete".to_string()
    } else {
        format!("Apply recovery: {}", parts.join("; "))
    }
}

fn summarize(items: &[String]) -> String {
    const LIMIT: usize = 3;
    let mut text = items
        .iter()
        .take(LIMIT)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if items.len() > LIMIT {
        text.push_str(", ...");
    }
    text
}
