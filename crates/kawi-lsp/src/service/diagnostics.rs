use super::*;
use crate::text::{self, line_character};
use crate::workspace::{self, path_from_uri};
use lsp_types::{
    DiagnosticSeverity as LspSeverity, NumberOrString, Position, PositionEncodingKind,
    PublishDiagnosticsParams,
};
use ropey::Rope;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticRelatedInformation {
    pub path: PathBuf,
    pub line: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: DiagnosticRange,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
    pub code_description: Option<String>,
    pub tags: Vec<DiagnosticTag>,
    pub related: Vec<DiagnosticRelatedInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsEvent {
    pub server: String,
    pub identifier: Option<String>,
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
        let diagnostics = convert_diagnostics(&params.diagnostics, &document.text, &encoding);
        self.emit(LspEvent::Diagnostics(DiagnosticsEvent {
            server: key.server,
            identifier: None,
            path,
            version: params.version,
            diagnostics,
        }));
    }

    pub(super) fn diagnostic_identifier(&self, key: &ClientKey) -> Option<String> {
        self.clients
            .get(key)
            .and_then(|client| client.diagnostic_identifier.clone())
    }
}

fn is_current_version(document_version: i32, published: Option<i32>) -> bool {
    published.is_none_or(|version| version == document_version)
}

pub(super) fn convert_diagnostics(
    diagnostics: &[lsp_types::Diagnostic],
    text: &Rope,
    encoding: &PositionEncodingKind,
) -> Vec<Diagnostic> {
    let lines = LineIndex::new(text);
    diagnostics
        .iter()
        .map(|diagnostic| convert_diagnostic(diagnostic, &lines, encoding))
        .collect()
}

struct LineIndex<'a> {
    rope: &'a Rope,
}

impl<'a> LineIndex<'a> {
    fn new(rope: &'a Rope) -> Self {
        Self { rope }
    }

    fn position(&self, position: Position, encoding: &PositionEncodingKind) -> DiagnosticPosition {
        let line = text::clamped_line(self.rope, position.line);
        DiagnosticPosition {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            character: line_character(self.rope, position.line, position.character, encoding),
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
        code_description: diagnostic
            .code_description
            .as_ref()
            .map(|description| description.href.to_string()),
        tags: diagnostic
            .tags
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(|tag| {
                if *tag == lsp_types::DiagnosticTag::UNNECESSARY {
                    Some(DiagnosticTag::Unnecessary)
                } else if *tag == lsp_types::DiagnosticTag::DEPRECATED {
                    Some(DiagnosticTag::Deprecated)
                } else {
                    None
                }
            })
            .collect(),
        related: diagnostic
            .related_information
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(related_information)
            .collect(),
    }
}

fn related_information(
    information: &lsp_types::DiagnosticRelatedInformation,
) -> Option<DiagnosticRelatedInformation> {
    let path = path_from_uri(&information.location.uri)?;
    Some(DiagnosticRelatedInformation {
        path: workspace::absolute(&path),
        line: information.location.range.start.line,
        message: information.message.clone(),
    })
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
    fn converts_tags_and_related_information() {
        let text = Rope::from_str("a\n");
        let lines = LineIndex::new(&text);
        let mut value = diagnostic(Range::new(Position::new(0, 0), Position::new(0, 1)));
        value.tags = Some(vec![lsp_types::DiagnosticTag::UNNECESSARY]);
        value.related_information = Some(vec![lsp_types::DiagnosticRelatedInformation {
            location: lsp_types::Location {
                uri: crate::workspace::file_uri(Path::new("/tmp/related.rs")).expect("uri"),
                range: Range::new(Position::new(3, 2), Position::new(3, 4)),
            },
            message: "related".to_string(),
        }]);
        let converted = convert_diagnostic(&value, &lines, &PositionEncodingKind::UTF16);
        assert_eq!(converted.tags, vec![DiagnosticTag::Unnecessary]);
        assert_eq!(converted.related.len(), 1);
        assert_eq!(converted.related[0].path, PathBuf::from("/tmp/related.rs"));
        assert_eq!(converted.related[0].line, 3);
        assert_eq!(converted.related[0].message, "related");
    }

    #[test]
    fn converts_code_descriptions_and_ignores_unknown_tags() {
        let text = Rope::from_str("a\n");
        let lines = LineIndex::new(&text);
        let mut value = diagnostic(Range::new(Position::new(0, 0), Position::new(0, 1)));
        value.code_description = Some(lsp_types::CodeDescription {
            href: "https://example.com/error".parse().expect("uri"),
        });
        value.tags = Some(vec![lsp_types::DiagnosticTag::DEPRECATED]);
        let converted = convert_diagnostic(&value, &lines, &PositionEncodingKind::UTF16);
        assert_eq!(
            converted.code_description.as_deref(),
            Some("https://example.com/error")
        );
        assert_eq!(converted.tags, vec![DiagnosticTag::Deprecated]);
    }

    #[test]
    fn missing_severity_defaults_to_error_and_unknown_tags_are_ignored() {
        let text = Rope::from_str("a\n");
        let lines = LineIndex::new(&text);
        let value: LspDiagnostic = serde_json::from_value(serde_json::json!({
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 1 }
            },
            "message": "message",
            "tags": [99]
        }))
        .expect("diagnostic");
        let converted = convert_diagnostic(&value, &lines, &PositionEncodingKind::UTF16);
        assert_eq!(converted.severity, DiagnosticSeverity::Error);
        assert!(converted.tags.is_empty());
    }

    #[test]
    fn converts_utf16_ranges_to_character_columns() {
        let text = Rope::from_str("a𝄞b\nsecond\r\n");
        let lines = LineIndex::new(&text);
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
        let text = Rope::from_str("a𝄞b");
        let lines = LineIndex::new(&text);
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
        let text = Rope::from_str("short");
        let lines = LineIndex::new(&text);
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
