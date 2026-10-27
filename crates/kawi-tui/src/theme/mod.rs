use crossterm::style::Color;
use std::path::Path;

pub use style::{SyntaxStyle, SyntaxStyles};

mod builtin;
mod color;
mod custom;
mod error;
mod style;

pub use error::ThemeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseColors {
    pub foreground: Color,
    pub background: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchColors {
    pub foreground: Color,
    pub background: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub base: BaseColors,
    pub float_background: Color,
    pub code_background: Color,
    pub search_match: MatchColors,
    pub search_current: MatchColors,
    pub substitute: MatchColors,
    pub selection: MatchColors,
    pub bracket_match: MatchColors,
    pub status: MatchColors,
    pub line_number: Color,
    pub non_text: Color,
    pub git_add: Color,
    pub git_change: Color,
    pub git_delete: Color,
    pub git_staged_add: Color,
    pub git_staged_change: Color,
    pub git_staged_delete: Color,
    pub diagnostic_error: Color,
    pub diagnostic_warning: Color,
    pub diagnostic_information: Color,
    pub diagnostic_hint: Color,
    pub syntax: SyntaxStyles,
}

impl Theme {
    pub fn load(name: &str, themes_dir: &Path) -> Result<Self, ThemeError> {
        match Self::from_file(name, themes_dir) {
            Ok(theme) => Ok(theme),
            Err(error) if error.is_not_found() => Self::named(name).ok_or(error),
            Err(error) => match Self::named(name) {
                Some(theme) => Ok(theme),
                None => Err(error),
            },
        }
    }

    pub fn validate(name: &str, themes_dir: &Path) -> Result<(), ThemeError> {
        match Self::from_file(name, themes_dir) {
            Ok(_) => Ok(()),
            Err(error) if error.is_not_found() => Self::named(name).map(|_| ()).ok_or(error),
            Err(error) => Err(error),
        }
    }

    pub fn sign_color(&self, sign: crate::frame::Sign) -> Color {
        match sign {
            crate::frame::Sign::Git(sign) => self.git_sign_color(sign),
            crate::frame::Sign::Diagnostic(severity) => self.diagnostic_color(severity),
        }
    }

    pub fn diagnostic_color(&self, severity: crate::frame::DiagnosticSeverity) -> Color {
        match severity {
            crate::frame::DiagnosticSeverity::Error => self.diagnostic_error,
            crate::frame::DiagnosticSeverity::Warning => self.diagnostic_warning,
            crate::frame::DiagnosticSeverity::Information => self.diagnostic_information,
            crate::frame::DiagnosticSeverity::Hint => self.diagnostic_hint,
        }
    }

    fn git_sign_color(&self, sign: crate::frame::GitSign) -> Color {
        match sign {
            crate::frame::GitSign::Add | crate::frame::GitSign::Untracked => self.git_add,
            crate::frame::GitSign::Change | crate::frame::GitSign::ChangeDelete => self.git_change,
            crate::frame::GitSign::Delete | crate::frame::GitSign::TopDelete => self.git_delete,
            crate::frame::GitSign::StagedAdd => self.git_staged_add,
            crate::frame::GitSign::StagedChange | crate::frame::GitSign::StagedChangeDelete => {
                self.git_staged_change
            }
            crate::frame::GitSign::StagedDelete | crate::frame::GitSign::StagedTopDelete => {
                self.git_staged_delete
            }
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}
