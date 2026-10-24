use crossterm::style::Color;
use kawi_syntax::SyntaxGroup;
use std::path::Path;

mod builtin;
mod color;
mod custom;
mod error;

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
pub struct SyntaxColors {
    comment: Color,
    keyword: Color,
    string: Color,
    number: Color,
    constant: Color,
    function: Color,
    type_name: Color,
    variable: Color,
    property: Color,
    operator: Color,
    punctuation: Color,
    attribute: Color,
    tag: Color,
    invalid: Color,
}

impl SyntaxColors {
    pub fn color(&self, group: SyntaxGroup) -> Color {
        match group {
            SyntaxGroup::Comment => self.comment,
            SyntaxGroup::Keyword => self.keyword,
            SyntaxGroup::String => self.string,
            SyntaxGroup::Number => self.number,
            SyntaxGroup::Constant => self.constant,
            SyntaxGroup::Function => self.function,
            SyntaxGroup::Type => self.type_name,
            SyntaxGroup::Variable => self.variable,
            SyntaxGroup::Property => self.property,
            SyntaxGroup::Operator => self.operator,
            SyntaxGroup::Punctuation => self.punctuation,
            SyntaxGroup::Attribute => self.attribute,
            SyntaxGroup::Tag => self.tag,
            SyntaxGroup::Invalid => self.invalid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub base: BaseColors,
    pub search_match: MatchColors,
    pub search_current: MatchColors,
    pub substitute: MatchColors,
    pub selection: MatchColors,
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
    pub syntax: SyntaxColors,
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
