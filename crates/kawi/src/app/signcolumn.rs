use kawi_lsp::DiagnosticSeverity;
use kawi_tui::frame::{DiagnosticSeverity as UiDiagnosticSeverity, Sign};

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
        self.apply_diagnostic_signs(&mut signs, rows);
        Some(signs)
    }

    fn has_signs(&self) -> bool {
        self.lsp.has_diagnostics()
            || (self.git_signs && self.git.as_ref().is_some_and(|session| session.has_signs()))
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
        for (sign, row) in signs.iter_mut().zip(rows) {
            let diagnostic = self
                .lsp
                .diagnostic_severity(*row)
                .map(ui_diagnostic_severity);
            *sign = preferred_sign(*sign, diagnostic);
        }
    }
}

fn preferred_sign(git: Option<Sign>, diagnostic: Option<UiDiagnosticSeverity>) -> Option<Sign> {
    diagnostic.map(Sign::Diagnostic).or(git)
}

fn ui_diagnostic_severity(severity: DiagnosticSeverity) -> UiDiagnosticSeverity {
    match severity {
        DiagnosticSeverity::Error => UiDiagnosticSeverity::Error,
        DiagnosticSeverity::Warning => UiDiagnosticSeverity::Warning,
        DiagnosticSeverity::Information => UiDiagnosticSeverity::Information,
        DiagnosticSeverity::Hint => UiDiagnosticSeverity::Hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use crate::options::SignColumnMode;
    use kawi_buffer::Buffer;
    use kawi_git::GitFile;
    use kawi_tui::frame::GitSign;
    use std::path::PathBuf;

    use crate::app::git::GitSession;

    fn app_with_untracked_file() -> App {
        let file = GitFile {
            root: PathBuf::from("."),
            path: PathBuf::from("script.py"),
            relpath: "script.py".to_string(),
            mode: "100644".to_string(),
            untracked: true,
            crlf: false,
            no_final_newline: false,
            head_no_final_newline: false,
            base: Vec::new(),
            head: Vec::new(),
        };
        let lines = vec!["print('hello')".to_string()];
        let session = GitSession::new(file, &lines, 1, false);
        let mut app = App::new(Buffer::from_string("print('hello')"), Flavor::Vim);
        app.handle_resize(80, 24).expect("resize");
        app.git = Some(session);
        app
    }

    #[test]
    fn signcolumn_defaults_to_hidden() {
        let app = app_with_untracked_file();
        assert_eq!(app.signcolumn, SignColumnMode::Never);
        assert!(!app.git_signs);
        assert!(app.view_signs().is_none());
    }

    #[test]
    fn never_hides_git_signs_even_when_enabled() {
        let mut app = app_with_untracked_file();
        app.git_signs = true;
        app.update_signcolumn_width();
        assert_eq!(app.editor.signcolumn_width(), 0);
        assert!(app.view_signs().is_none());
    }

    #[test]
    fn auto_shows_git_signs_only_when_enabled_and_present() {
        let mut app = app_with_untracked_file();
        app.signcolumn = SignColumnMode::Auto;
        app.update_signcolumn_width();
        assert_eq!(app.editor.signcolumn_width(), 0);
        assert!(app.view_signs().is_none());

        app.git_signs = true;
        app.update_signcolumn_width();
        let signs = app.view_signs().expect("signs");
        assert_eq!(app.editor.signcolumn_width(), SIGN_COLUMN_WIDTH);
        assert_eq!(signs[0], Some(Sign::Git(GitSign::Untracked)));
    }

    #[test]
    fn always_reserves_the_column_without_signs() {
        let mut app = app_with_untracked_file();
        app.signcolumn = SignColumnMode::Always;
        app.update_signcolumn_width();
        let signs = app.view_signs().expect("signs");
        assert_eq!(app.editor.signcolumn_width(), SIGN_COLUMN_WIDTH);
        assert!(signs.iter().all(Option::is_none));
    }

    #[test]
    fn diagnostics_take_priority_over_git_signs() {
        let git = Some(Sign::Git(GitSign::Change));
        let diagnostic = Some(UiDiagnosticSeverity::Error);
        assert_eq!(
            preferred_sign(git, diagnostic),
            Some(Sign::Diagnostic(UiDiagnosticSeverity::Error))
        );
        assert_eq!(preferred_sign(git, None), git);
    }
}
