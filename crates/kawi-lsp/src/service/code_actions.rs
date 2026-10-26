use super::text_requests::{PendingTextRequest, TextResponse};
use super::*;
use crate::text;
use lsp_types::{
    CodeActionOrCommand, CodeActionResponse, DiagnosticSeverity as LspSeverity, NumberOrString,
};
use ropey::Rope;
use serde_json::Value;

impl Manager {
    pub(super) fn code_action_event(
        &self,
        pending: &PendingTextRequest,
        responses: &[TextResponse],
    ) -> Result<CodeActionEvent, String> {
        let mut actions = Vec::new();
        let mut response_error = None;
        for response in responses {
            match self.code_action_response(
                response.value.clone(),
                &response.server,
                &response.encoding,
            ) {
                Ok(mut entries) => actions.append(&mut entries),
                Err(error) => {
                    response_error.get_or_insert(error);
                }
            }
        }
        if actions.is_empty() {
            return Err(response_error.unwrap_or_else(|| "No code actions available".to_string()));
        }
        actions.sort_by_key(|entry| !entry.preferred);
        Ok(CodeActionEvent {
            request: pending.request,
            path: pending.path.clone(),
            actions,
        })
    }

    fn code_action_response(
        &self,
        value: Value,
        server: &str,
        encoding: &PositionEncodingKind,
    ) -> Result<Vec<CodeActionEntry>, String> {
        if value.is_null() {
            return Err("No code actions available".to_string());
        }
        let response = serde_json::from_value::<CodeActionResponse>(value)
            .map_err(|error| format!("Invalid code action response: {error}"))?;
        let mut actions = Vec::new();
        let mut edit_error = None;
        for action in response {
            match self.code_action_entry(action, server, encoding) {
                Ok(Some(entry)) => actions.push(entry),
                Ok(None) => {}
                Err(error) => {
                    edit_error.get_or_insert(error);
                }
            }
        }
        if actions.is_empty() {
            return Err(edit_error.unwrap_or_else(|| "No code actions available".to_string()));
        }
        Ok(actions)
    }

    fn code_action_entry(
        &self,
        action: CodeActionOrCommand,
        server: &str,
        encoding: &PositionEncodingKind,
    ) -> Result<Option<CodeActionEntry>, String> {
        match action {
            CodeActionOrCommand::Command(command) => {
                let command = super::convert_command(command);
                if command.title.is_empty() {
                    return Ok(None);
                }
                Ok(Some(CodeActionEntry {
                    server: server.to_string(),
                    title: command.title.clone(),
                    kind: None,
                    preferred: false,
                    edit: None,
                    command: Some(command),
                }))
            }
            CodeActionOrCommand::CodeAction(action) => {
                if action.disabled.is_some() {
                    return Ok(None);
                }
                let title = one_line_message(&action.title);
                if title.is_empty() {
                    return Ok(None);
                }
                let edit = match action.edit {
                    Some(edit) => Some(
                        self.parse_workspace_edit(
                            serde_json::to_value(edit)
                                .map_err(|error| format!("Invalid code action edit: {error}"))?,
                            encoding,
                        )?,
                    ),
                    None => None,
                };
                let command = action.command.map(super::convert_command);
                if edit.is_none() && command.is_none() {
                    return Ok(None);
                }
                Ok(Some(CodeActionEntry {
                    server: server.to_string(),
                    title,
                    kind: action
                        .kind
                        .as_ref()
                        .map(|kind| one_line_message(kind.as_str())),
                    preferred: action.is_preferred.unwrap_or(false),
                    edit,
                    command,
                }))
            }
        }
    }
}

pub(super) fn protocol_diagnostics(
    diagnostics: &[Diagnostic],
    text: &Rope,
    encoding: &PositionEncodingKind,
) -> Vec<lsp_types::Diagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| lsp_types::Diagnostic {
            range: protocol_range(diagnostic.range, text, encoding),
            severity: Some(protocol_severity(diagnostic.severity)),
            code: diagnostic.code.as_deref().map(protocol_code),
            code_description: None,
            source: diagnostic.source.clone(),
            message: diagnostic.message.clone(),
            ..lsp_types::Diagnostic::default()
        })
        .collect()
}

fn protocol_code(code: &str) -> NumberOrString {
    code.parse::<i32>()
        .map(NumberOrString::Number)
        .unwrap_or_else(|_| NumberOrString::String(code.to_string()))
}

fn protocol_range(
    range: DiagnosticRange,
    text: &Rope,
    encoding: &PositionEncodingKind,
) -> lsp_types::Range {
    lsp_types::Range::new(
        text::position_from_column(text, range.start.line, range.start.character, encoding),
        text::position_from_column(text, range.end.line, range.end.character, encoding),
    )
}

fn protocol_severity(severity: DiagnosticSeverity) -> LspSeverity {
    match severity {
        DiagnosticSeverity::Error => LspSeverity::ERROR,
        DiagnosticSeverity::Warning => LspSeverity::WARNING,
        DiagnosticSeverity::Information => LspSeverity::INFORMATION,
        DiagnosticSeverity::Hint => LspSeverity::HINT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Position, Range};

    #[test]
    fn converts_diagnostics_to_the_negotiated_encoding() {
        let text = Rope::from_str("a𝄞b\n");
        let diagnostics = vec![Diagnostic {
            range: DiagnosticRange {
                start: DiagnosticPosition {
                    line: 0,
                    character: 1,
                },
                end: DiagnosticPosition {
                    line: 0,
                    character: 2,
                },
            },
            severity: DiagnosticSeverity::Warning,
            message: "message".to_string(),
            source: Some("fake".to_string()),
            code: Some("W1".to_string()),
            code_description: None,
            tags: Vec::new(),
            related: Vec::new(),
        }];
        let converted = protocol_diagnostics(&diagnostics, &text, &PositionEncodingKind::UTF16);
        assert_eq!(
            converted[0].range,
            Range::new(Position::new(0, 1), Position::new(0, 3))
        );
        assert_eq!(converted[0].severity, Some(LspSeverity::WARNING));
        assert_eq!(
            converted[0].code,
            Some(NumberOrString::String("W1".to_string()))
        );
        assert_eq!(protocol_code("7"), NumberOrString::Number(7));
    }
}
