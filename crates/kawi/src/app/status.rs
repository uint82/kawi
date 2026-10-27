use std::path::Path;
use std::time::Duration;

use kawi_editor::Mode;
use kawi_tui::frame::DiagnosticSeverity;
use kawi_tui::statusbar::ModeKind;
use kawi_tui::text::count_label;

use super::diagnostic_index::DiagnosticCounts;
use super::App;

const SPINNER: [&str; 4] = ["-", "\\", "|", "/"];
const SPINNER_INTERVAL: Duration = Duration::from_millis(120);

pub(crate) struct StatusDiagnostic {
    pub text: String,
    pub severity: DiagnosticSeverity,
}

impl App {
    pub(super) fn status_mode_kind(&self) -> ModeKind {
        self.editor.mode().into()
    }

    pub(super) fn status_filetype(&self) -> Option<&'static str> {
        self.editor.language_id().map(|language| language.name())
    }

    pub(super) fn status_indent(&self) -> String {
        format_indent(self.editor.expandtab(), self.editor.shiftwidth())
    }

    pub(super) fn status_selection(&self) -> Option<String> {
        let ((start_row, start_col), (end_row, end_col)) = self.editor.selection_range()?;
        match self.editor.mode() {
            Mode::VisualLine => Some(count_label(
                end_row.saturating_sub(start_row).saturating_add(1),
                "line",
                "lines",
            )),
            Mode::VisualBlock => Some(format!(
                "{}x{}",
                end_row.saturating_sub(start_row).saturating_add(1),
                end_col.saturating_sub(start_col).saturating_add(1)
            )),
            Mode::Visual => Some(count_label(
                self.selection_chars(start_row, start_col, end_row, end_col),
                "char",
                "chars",
            )),
            _ => None,
        }
    }

    pub(super) fn status_percent(&self) -> Option<usize> {
        let lines = self.editor.line_count();
        if lines == 0 {
            return None;
        }
        let row = self.editor.row().min(lines.saturating_sub(1));
        Some(
            row.saturating_add(1)
                .saturating_mul(100)
                .checked_div(lines)
                .unwrap_or(0)
                .min(100),
        )
    }

    pub(super) fn status_spinner_timeout(&self) -> Option<Duration> {
        self.lsp.has_progress().then_some(SPINNER_INTERVAL)
    }

    pub(super) fn advance_status_spinner(&mut self) {
        self.status_spinner = self.status_spinner.wrapping_add(1);
    }

    pub(super) fn status_lsp(&self) -> Option<String> {
        let status = self.lsp.status()?;
        if !self.lsp.has_progress() {
            return Some(status.to_string());
        }
        let spinner = SPINNER[self.status_spinner % SPINNER.len()];
        Some(format!("{spinner} {status}"))
    }

    pub(super) fn status_diagnostics(&self, path: Option<&Path>) -> Option<StatusDiagnostic> {
        let path = path?;
        let counts = self.problems.diagnostic_counts(path);
        let text = self.diagnostic_options.format_counts(counts)?;
        let severity = max_severity(counts)?;
        Some(StatusDiagnostic { text, severity })
    }

    pub(super) fn status_workspace_diagnostics(&self) -> Option<StatusDiagnostic> {
        if !self.diagnostic_options.workspace_counts {
            return None;
        }
        let counts = self.problems.workspace_diagnostic_counts();
        let text = self.diagnostic_options.format_counts(counts)?;
        let severity = max_severity(counts)?;
        Some(StatusDiagnostic {
            text: format!("workspace {text}"),
            severity,
        })
    }

    fn selection_chars(
        &self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) -> usize {
        (start_row..=end_row).fold(0, |total, row| {
            let start = if row == start_row { start_col } else { 0 };
            let end = if row == end_row {
                end_col
            } else {
                self.editor.line_len(row)
            };
            total.saturating_add(end.saturating_sub(start))
        })
    }
}

fn format_indent(expandtab: bool, shiftwidth: usize) -> String {
    if expandtab {
        format!("{shiftwidth} spaces")
    } else {
        "tabs".to_string()
    }
}

fn max_severity(counts: DiagnosticCounts) -> Option<DiagnosticSeverity> {
    if counts.errors > 0 {
        Some(DiagnosticSeverity::Error)
    } else if counts.warnings > 0 {
        Some(DiagnosticSeverity::Warning)
    } else if counts.information > 0 {
        Some(DiagnosticSeverity::Information)
    } else if counts.hints > 0 {
        Some(DiagnosticSeverity::Hint)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;

    #[test]
    fn spinner_timeout_tracks_lsp_progress() {
        let mut app = App::new(Buffer::from_string("x\n"), Flavor::Vim);
        assert!(app.status_spinner_timeout().is_none());
        app.lsp.set_progress_for_test();
        assert!(app.status_spinner_timeout().is_some());
        app.advance_status_spinner();
        assert_eq!(app.status_spinner, 1);
    }

    #[test]
    fn indentation_describes_the_detected_unit() {
        assert_eq!(format_indent(true, 4), "4 spaces");
        assert_eq!(format_indent(false, 4), "tabs");
    }
}
