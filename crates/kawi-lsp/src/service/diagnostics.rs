use super::*;
use crate::workspace::{self, char_index, path_from_uri};
use lsp_types::{
    DiagnosticSeverity as LspSeverity, NumberOrString, Position, PositionEncodingKind,
    PublishDiagnosticsParams,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiagnosticPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticRange {
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: DiagnosticRange,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsEvent {
    pub server: String,
    pub path: PathBuf,
    pub version: Option<i32>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Manager {
    pub(super) fn publish_diagnostics(&mut self, key: ClientKey, params: PublishDiagnosticsParams) {
        let Some(path) = path_from_uri(&params.uri) else {
            return;
        };
        let path = workspace::absolute(&path);
        let Some(document) = self.documents.get(&path) else {
            return;
        };
        if !is_current_version(document.version, params.version) {
            return;
        }
        let encoding = self
            .clients
            .get(&key)
            .map_or(PositionEncodingKind::UTF16, |client| {
                client.encoding.clone()
            });
        let lines = LineIndex::new(&document.text);
        let diagnostics = params
            .diagnostics
            .iter()
            .map(|diagnostic| convert_diagnostic(diagnostic, &lines, &encoding))
            .collect();
        self.emit(LspEvent::Diagnostics(DiagnosticsEvent {
            server: key.server,
            path,
            version: params.version,
            diagnostics,
        }));
    }
}

fn is_current_version(document_version: i32, published: Option<i32>) -> bool {
    published.is_none_or(|version| version == document_version)
}

struct LineIndex<'a> {
    lines: Vec<&'a str>,
}

impl<'a> LineIndex<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            lines: text
                .split('\n')
                .map(|line| line.strip_suffix('\r').unwrap_or(line))
                .collect(),
        }
    }

    fn position(&self, position: Position, encoding: &PositionEncodingKind) -> DiagnosticPosition {
        let line = usize::try_from(position.line)
            .unwrap_or(usize::MAX)
            .min(self.lines.len().saturating_sub(1));
        let text = self.lines.get(line).copied().unwrap_or("");
        DiagnosticPosition {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            character: char_index(text, position.character, encoding),
        }
    }
}

fn convert_diagnostic(
    diagnostic: &lsp_types::Diagnostic,
    lines: &LineIndex,
    encoding: &PositionEncodingKind,
) -> Diagnostic {
    let start = lines.position(diagnostic.range.start, encoding);
    let mut end = lines.position(diagnostic.range.end, encoding);
    if end < start {
        end = start;
    }
    Diagnostic {
        range: DiagnosticRange { start, end },
        severity: severity(diagnostic.severity),
        message: diagnostic.message.clone(),
        source: diagnostic.source.clone(),
        code: diagnostic.code.as_ref().map(code_string),
    }
}

fn severity(severity: Option<LspSeverity>) -> DiagnosticSeverity {
    match severity {
        Some(level) if level == LspSeverity::WARNING => DiagnosticSeverity::Warning,
        Some(level) if level == LspSeverity::INFORMATION => DiagnosticSeverity::Information,
        Some(level) if level == LspSeverity::HINT => DiagnosticSeverity::Hint,
        _ => DiagnosticSeverity::Error,
    }
}

fn code_string(code: &NumberOrString) -> String {
    match code {
        NumberOrString::Number(value) => value.to_string(),
        NumberOrString::String(value) => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Diagnostic as LspDiagnostic, Range};

    fn diagnostic(range: Range) -> LspDiagnostic {
        LspDiagnostic {
            range,
            severity: Some(LspSeverity::WARNING),
            message: "message".to_string(),
            source: Some("fake".to_string()),
            code: Some(NumberOrString::Number(7)),
            ..LspDiagnostic::default()
        }
    }

    #[test]
    fn only_the_current_document_version_is_accepted() {
        assert!(is_current_version(4, Some(4)));
        assert!(!is_current_version(4, Some(3)));
        assert!(!is_current_version(4, Some(5)));
        assert!(is_current_version(4, None));
    }

    #[test]
    fn converts_utf16_ranges_to_character_columns() {
        let lines = LineIndex::new("a𝄞b\nsecond\r\n");
        let value = diagnostic(Range::new(Position::new(0, 1), Position::new(0, 3)));
        let converted = convert_diagnostic(&value, &lines, &PositionEncodingKind::UTF16);
        assert_eq!(
            converted.range.start,
            DiagnosticPosition {
                line: 0,
                character: 1
            }
        );
        assert_eq!(
            converted.range.end,
            DiagnosticPosition {
                line: 0,
                character: 2
            }
        );
        assert_eq!(converted.severity, DiagnosticSeverity::Warning);
        assert_eq!(converted.code.as_deref(), Some("7"));
    }

    #[test]
    fn converts_utf8_ranges_to_character_columns() {
        let lines = LineIndex::new("a𝄞b");
        let value = diagnostic(Range::new(Position::new(0, 1), Position::new(0, 5)));
        let converted = convert_diagnostic(&value, &lines, &PositionEncodingKind::UTF8);
        assert_eq!(
            converted.range.start,
            DiagnosticPosition {
                line: 0,
                character: 1
            }
        );
        assert_eq!(
            converted.range.end,
            DiagnosticPosition {
                line: 0,
                character: 2
            }
        );
    }

    #[test]
    fn clamps_positions_beyond_the_document() {
        let lines = LineIndex::new("short");
        let value = diagnostic(Range::new(Position::new(9, 99), Position::new(9, 100)));
        let converted = convert_diagnostic(&value, &lines, &PositionEncodingKind::UTF16);
        assert_eq!(
            converted.range.start,
            DiagnosticPosition {
                line: 0,
                character: 5
            }
        );
        assert_eq!(converted.range.end, converted.range.start);
    }
}
