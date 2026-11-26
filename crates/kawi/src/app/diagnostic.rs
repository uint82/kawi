use std::fmt;

use kawi_tui::frame::MessageSeverity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiagnosticCode(&'static str);

impl DiagnosticCode {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self(code)
    }

    pub(crate) const fn as_str(self) -> &'static str {
        self.0
    }
}

impl From<&'static str> for DiagnosticCode {
    fn from(code: &'static str) -> Self {
        Self::new(code)
    }
}

impl From<kawi_editor::command::DiagnosticCode> for DiagnosticCode {
    fn from(code: kawi_editor::command::DiagnosticCode) -> Self {
        Self::new(code.as_str())
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DiagnosticSource {
    #[default]
    Builtin,
    LanguageServer,
    Task,
    Git,
    Plugin,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Diagnostic {
    code: Option<DiagnosticCode>,
    severity: MessageSeverity,
    source: DiagnosticSource,
    message: String,
    help: Option<String>,
}

impl Diagnostic {
    fn new(severity: MessageSeverity, message: impl Into<String>) -> Self {
        Self {
            code: None,
            severity,
            source: DiagnosticSource::default(),
            message: message.into(),
            help: None,
        }
    }

    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self::new(MessageSeverity::Error, message)
    }

    pub(crate) fn no_file_name() -> Self {
        Self::error("No file name").with_code("E32")
    }

    pub(crate) fn warning(message: impl Into<String>) -> Self {
        Self::new(MessageSeverity::Warning, message)
    }

    pub(crate) fn information(message: impl Into<String>) -> Self {
        Self::new(MessageSeverity::Information, message)
    }

    pub(crate) fn success(message: impl Into<String>) -> Self {
        Self::new(MessageSeverity::Success, message)
    }

    pub(crate) fn with_code(mut self, code: impl Into<DiagnosticCode>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub(crate) fn with_source(mut self, source: DiagnosticSource) -> Self {
        self.source = source;
        self
    }

    pub(crate) fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub(crate) const fn severity(&self) -> MessageSeverity {
        self.severity
    }

    pub(crate) const fn code(&self) -> Option<DiagnosticCode> {
        self.code
    }

    #[cfg(test)]
    pub(crate) const fn source(&self) -> DiagnosticSource {
        self.source
    }

    pub(crate) fn render(&self) -> String {
        let mut text = String::with_capacity(
            self.message.len() + self.help.as_ref().map_or(0, String::len) + 8,
        );
        if let Some(code) = self.code() {
            text.push_str(code.as_str());
            text.push_str(": ");
        }
        text.push_str(&self.message);
        if let Some(help) = self.help.as_deref().filter(|help| !help.is_empty()) {
            text.push(' ');
            text.push_str(help);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_code_and_message() {
        let diagnostic = Diagnostic::error("No file name").with_code("E32");
        assert_eq!(diagnostic.render(), "E32: No file name");
        assert_eq!(diagnostic.severity(), MessageSeverity::Error);
        assert_eq!(diagnostic.code().map(DiagnosticCode::as_str), Some("E32"));
    }

    #[test]
    fn renders_help_after_the_message() {
        let diagnostic = Diagnostic::error("Not an editor command: hlep")
            .with_code("E492")
            .with_help("Did you mean :help?");
        assert_eq!(
            diagnostic.render(),
            "E492: Not an editor command: hlep Did you mean :help?"
        );
    }

    #[test]
    fn omits_empty_help() {
        let diagnostic = Diagnostic::warning("careful").with_help("");
        assert_eq!(diagnostic.render(), "careful");
    }

    #[test]
    fn defaults_to_builtin_source() {
        assert_eq!(
            Diagnostic::information("saved").source(),
            DiagnosticSource::Builtin
        );
        assert_eq!(
            Diagnostic::information("saved")
                .with_source(DiagnosticSource::Plugin)
                .source(),
            DiagnosticSource::Plugin
        );
    }

    #[test]
    fn builds_the_shared_no_file_name_error() {
        assert_eq!(Diagnostic::no_file_name().render(), "E32: No file name");
    }

    #[test]
    fn severity_is_not_inferred_from_text() {
        let diagnostic = Diagnostic::information("E32: No file name");
        assert_eq!(diagnostic.severity(), MessageSeverity::Information);
        assert_eq!(diagnostic.render(), "E32: No file name");
    }

    #[test]
    fn accepts_a_static_code() {
        assert_eq!(
            Diagnostic::error("nope").with_code("E1").render(),
            "E1: nope"
        );
    }

    #[test]
    fn renders_without_a_code() {
        assert_eq!(Diagnostic::success("mapped x y").render(), "mapped x y");
    }
}
