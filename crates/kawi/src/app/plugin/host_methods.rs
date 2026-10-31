use kawi_buffer::{char_offset_at, validate_text_edits, TextEdit};
use kawi_editor::Mode;
use std::path::{Path, PathBuf};

use kawi_plugin::{
    validate_diagnostics, ApplyEditParams, ApplyEditResult, DiagnosticSummary, GetStateParams,
    GetStateResult, InvocationContext, Position, RpcError, Selection, SetDiagnosticsParams,
    UiMessageParams, INTERNAL_ERROR,
};
use ropey::Rope;
use serde_json::Value;

use super::super::problems::{plugin_source, Problem, ProblemSeverity};
use super::super::App;

impl App {
    pub(crate) fn handle_plugin_request(
        &mut self,
        plugin: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        match method {
            "editor.get_state" => self.plugin_get_state(params),
            "editor.apply_edit" => self.plugin_apply_edit(params),
            "editor.set_diagnostics" => self.plugin_set_diagnostics(plugin, params),
            "ui.message" => self.plugin_message(plugin, params),
            _ => Err(RpcError::method_not_found(method)),
        }
    }

    pub(crate) fn plugin_context(&self) -> InvocationContext {
        let state = self.plugin_state(false);
        InvocationContext {
            buffer: state.buffer,
            revision: state.revision,
            path: state.path,
            language: state.language,
            mode: state.mode,
            cursor: state.cursor,
            selection: state.selection,
        }
    }

    fn plugin_get_state(&self, params: Value) -> Result<Value, RpcError> {
        let params: GetStateParams = serde_json::from_value(params).map_err(invalid_params)?;
        serde_json::to_value(self.plugin_state(params.include_text))
            .map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string()))
    }

    fn plugin_set_diagnostics(&mut self, plugin: &str, params: Value) -> Result<Value, RpcError> {
        if !self.plugins.allow_diagnostic_publish(plugin) {
            return Err(RpcError::invalid_params(
                "diagnostic publish rate limit exceeded",
            ));
        }
        let params: SetDiagnosticsParams =
            serde_json::from_value(params).map_err(invalid_params)?;
        let buffer = self.editor.buffer_number() as u64;
        if params.buffer != buffer {
            return Err(RpcError::stale_buffer(params.buffer, buffer));
        }
        let revision = self.editor.buffer_revision();
        if params.revision != revision {
            return Err(RpcError::stale_revision(params.revision, revision));
        }
        if !self.editor.has_file() {
            return Err(RpcError::invalid_params(
                "editor.set_diagnostics requires a named buffer",
            ));
        }
        validate_diagnostics(&params.diagnostics)?;
        let path = PathBuf::from(self.editor.filename());
        let snapshot = self.editor.document_snapshot();
        let diagnostics = params
            .diagnostics
            .iter()
            .map(|diagnostic| plugin_problem(&snapshot, plugin, &path, diagnostic))
            .collect::<Result<Vec<_>, _>>()?;
        self.problems.replace_plugin(plugin, &path, diagnostics);
        Ok(Value::Null)
    }

    fn plugin_message(&mut self, _plugin: &str, params: Value) -> Result<Value, RpcError> {
        let params: UiMessageParams = serde_json::from_value(params).map_err(invalid_params)?;
        let message = super::text::sanitize(&params.message);
        self.show_message(message);
        Ok(Value::Null)
    }

    fn plugin_apply_edit(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: ApplyEditParams = serde_json::from_value(params).map_err(invalid_params)?;
        let current_buffer = self.editor.buffer_number() as u64;
        if params.buffer != current_buffer {
            return Err(RpcError::stale_buffer(params.buffer, current_buffer));
        }
        let current = self.editor.buffer_revision();
        if params.revision != current {
            return Err(RpcError::stale_revision(params.revision, current));
        }
        let snapshot = self.editor.document_snapshot();
        let edits = protocol_edits(&snapshot, &params.edits)?;
        if edits.is_empty() {
            return Ok(serde_json::json!({"revision": current}));
        }
        validate_text_edits(snapshot.len_chars(), &edits)
            .map_err(|error| RpcError::invalid_params(format!("invalid text edits: {error:?}")))?;
        self.editor
            .apply_text_edits_at_revision(params.revision, &edits)
            .map_err(|error| match error {
                kawi_editor::EditError::StaleRevision => {
                    RpcError::stale_revision(params.revision, self.editor.buffer_revision())
                }
                other => RpcError::invalid_params(format!("cannot apply edits: {other:?}")),
            })?;
        self.lsp.sync_editor(&self.editor);
        self.reveal_cursor();
        serde_json::to_value(ApplyEditResult {
            revision: self.editor.buffer_revision(),
        })
        .map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string()))
    }

    fn plugin_state(&self, include_text: bool) -> GetStateResult {
        let snapshot = self.editor.document_snapshot();
        let selection =
            self.editor
                .selection_range()
                .map(|((start_row, start_col), (end_row, end_col))| Selection {
                    start: position_at(&snapshot, start_row, start_col),
                    end: position_at(&snapshot, end_row, end_col),
                });
        GetStateResult {
            buffer: self.editor.buffer_number() as u64,
            revision: self.editor.buffer_revision(),
            path: self
                .editor
                .has_file()
                .then(|| self.editor.filename().to_string())
                .filter(|path| !path.is_empty()),
            language: self
                .editor
                .language_id()
                .map(|language| language.name().to_string()),
            mode: mode_name(self.editor.mode()).to_string(),
            cursor: position_at(&snapshot, self.editor.row(), self.editor.col()),
            selection,
            text: include_text.then(|| snapshot.to_string()),
        }
    }
}

fn protocol_edits(
    snapshot: &Rope,
    edits: &[kawi_plugin::ProtocolTextEdit],
) -> Result<Vec<TextEdit>, RpcError> {
    edits
        .iter()
        .map(|edit| {
            let start = position_to_char(snapshot, &edit.start)?;
            let end = position_to_char(snapshot, &edit.end)?;
            if start > end {
                return Err(RpcError::invalid_position(
                    "edit start is after edit end".to_string(),
                ));
            }
            Ok(TextEdit {
                start,
                end,
                text: edit.text.clone(),
            })
        })
        .collect()
}

pub(super) fn position_to_char(snapshot: &Rope, position: &Position) -> Result<usize, RpcError> {
    char_offset_at(snapshot, position.line, position.byte_column)
        .map_err(|error| RpcError::invalid_position(error.to_string()))
}

fn position_at(snapshot: &Rope, line: usize, column: usize) -> Position {
    let line = line.min(snapshot.len_lines().saturating_sub(1));
    let text = snapshot.line(line);
    Position {
        line,
        byte_column: text.char_to_byte(column.min(text.len_chars())),
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "normal",
        Mode::Insert => "insert",
        Mode::Replace => "replace",
        Mode::Command => "command",
        Mode::Visual | Mode::VisualLine | Mode::VisualBlock => "visual",
        Mode::Search => "search",
    }
}

fn invalid_params(error: serde_json::Error) -> RpcError {
    RpcError::invalid_params(error.to_string())
}

fn plugin_problem(
    snapshot: &Rope,
    plugin: &str,
    path: &Path,
    summary: &DiagnosticSummary,
) -> Result<Problem, RpcError> {
    let start = position_to_char(snapshot, &summary.start)?;
    let end = position_to_char(snapshot, &summary.end)?;
    if start > end {
        return Err(RpcError::invalid_position(
            "diagnostic start is after its end".to_string(),
        ));
    }
    let (line, column) = display_position(snapshot, start);
    let (end_line, end_column) = display_position(snapshot, end);
    Ok(Problem {
        source: plugin_source(plugin),
        path: path.to_path_buf(),
        line,
        column,
        end_line,
        end_column,
        severity: plugin_severity(&summary.severity)?,
        message: super::text::sanitize(&summary.message),
        code: summary
            .code
            .as_ref()
            .map(|code| super::text::sanitize(code)),
        code_description: None,
        provider: Some(plugin.to_string()),
        tags: Vec::new(),
        related: Vec::new(),
    })
}

fn display_position(snapshot: &Rope, offset: usize) -> (usize, usize) {
    let line = snapshot.char_to_line(offset);
    let start = snapshot.line_to_char(line);
    (
        line.saturating_add(1),
        offset.saturating_sub(start).saturating_add(1),
    )
}

fn plugin_severity(value: &str) -> Result<ProblemSeverity, RpcError> {
    match value.to_ascii_lowercase().as_str() {
        "error" => Ok(ProblemSeverity::Error),
        "warning" | "warn" => Ok(ProblemSeverity::Warning),
        "information" | "info" => Ok(ProblemSeverity::Information),
        "hint" => Ok(ProblemSeverity::Hint),
        other => Err(RpcError::invalid_params(format!(
            "unknown diagnostic severity: {other}"
        ))),
    }
}
