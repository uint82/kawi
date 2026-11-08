use kawi_tui::frame::{DebugSign as UiDebugSign, DiagnosticSeverity as UiDiagnosticSeverity, Sign};

use super::problems::ProblemSeverity;
use super::App;

const SIGN_COLUMN_WIDTH: usize = 1;

impl App {
    pub(super) fn update_signcolumn_width(&mut self) {
        let has_signs = self.has_signs();
        let width = if self.signcolumn.is_visible(has_signs) {
            SIGN_COLUMN_WIDTH
        } else {
            0
        };
        self.editor.set_signcolumn_width(width);
    }

    #[cfg(test)]
    pub(super) fn view_signs(&self) -> Option<Vec<Option<Sign>>> {
        let top = self.editor.visible_top();
        let rows = self.editor.visible_rows(top, self.editor.view_height());
        self.view_signs_for_rows(&rows)
    }

    pub(super) fn view_signs_for_rows(&self, rows: &[usize]) -> Option<Vec<Option<Sign>>> {
        if !self.signcolumn.is_visible(self.has_signs()) {
            return None;
        }
        let mut signs = self
            .git_signs_for_rows(rows)
            .unwrap_or_else(|| vec![None; rows.len()]);
        self.apply_plugin_signs(&mut signs, rows);
        self.apply_diagnostic_signs(&mut signs, rows);
        self.apply_debug_signs(&mut signs, rows);
        self.apply_conflict_signs(&mut signs, rows);
        Some(signs)
    }

    fn has_signs(&self) -> bool {
        self.merge_conflict.has_blocks()
            || self.lsp_has_signs()
            || (self.git_signs && self.git.as_ref().is_some_and(|session| session.has_signs()))
            || self.plugin_has_signs()
            || self.debug_has_signs()
    }

    fn lsp_has_signs(&self) -> bool {
        super::lsp::editor_path(&self.editor)
            .is_some_and(|path| self.problems.diagnostic_counts(&path).highest().is_some())
    }

    fn debug_has_signs(&self) -> bool {
        super::lsp::editor_path(&self.editor).is_some_and(|path| self.debug.has_signs(&path))
    }

    fn plugin_has_signs(&self) -> bool {
        let buffer = self.editor.buffer_number();
        self.plugin_decorations.has_signs(buffer)
    }

    fn apply_plugin_signs(&self, signs: &mut [Option<Sign>], rows: &[usize]) {
        let buffer = self.editor.buffer_number();
        for (sign, plugin) in signs
            .iter_mut()
            .zip(self.plugin_decorations.signs(buffer, rows))
        {
            if plugin.is_some() {
                *sign = plugin;
            }
        }
    }

    fn git_signs_for_rows(&self, rows: &[usize]) -> Option<Vec<Option<Sign>>> {
        if !self.git_signs {
            return None;
        }
        self.git.as_ref().map(|session| {
            rows.iter()
                .map(|row| session.sign_at(*row).map(Sign::Git))
                .collect()
        })
    }

    fn apply_diagnostic_signs(&self, signs: &mut [Option<Sign>], rows: &[usize]) {
        let Some(path) = super::lsp::editor_path(&self.editor) else {
            return;
        };
        for (sign, row) in signs.iter_mut().zip(rows) {
            let diagnostic = self
                .problems
                .lsp_severity(&path, *row)
                .map(ui_diagnostic_severity);
            *sign = preferred_sign(sign.take(), diagnostic);
        }
    }

    fn apply_debug_signs(&self, signs: &mut [Option<Sign>], rows: &[usize]) {
        let Some(path) = super::lsp::editor_path(&self.editor) else {
            return;
        };
        for (sign, row) in signs.iter_mut().zip(rows) {
            if let Some(debug) = self.debug.sign_for(&path, *row) {
                *sign = Some(Sign::Debug(ui_debug_sign(debug)));
            }
        }
    }

    fn apply_conflict_signs(&self, signs: &mut [Option<Sign>], rows: &[usize]) {
        let blocks = self.merge_conflict.blocks();
        if blocks.is_empty() {
            return;
        }
        for (sign, row) in signs.iter_mut().zip(rows) {
            if blocks
                .binary_search_by_key(row, |block| block.start)
                .is_ok()
            {
                *sign = Some(Sign::Conflict);
            }
        }
    }
}

fn ui_debug_sign(sign: super::debug::DebugSign) -> UiDebugSign {
    match sign {
        super::debug::DebugSign::Breakpoint => UiDebugSign::Breakpoint,
        super::debug::DebugSign::Unverified => UiDebugSign::Unverified,
        super::debug::DebugSign::Current => UiDebugSign::Current,
    }
}

fn preferred_sign(git: Option<Sign>, diagnostic: Option<UiDiagnosticSeverity>) -> Option<Sign> {
    diagnostic.map(Sign::Diagnostic).or(git)
}

fn ui_diagnostic_severity(severity: ProblemSeverity) -> UiDiagnosticSeverity {
    match severity {
        ProblemSeverity::Error => UiDiagnosticSeverity::Error,
        ProblemSeverity::Warning => UiDiagnosticSeverity::Warning,
        ProblemSeverity::Information => UiDiagnosticSeverity::Information,
        ProblemSeverity::Hint => UiDiagnosticSeverity::Hint,
    }
}

#[cfg(test)]
mod tests;
