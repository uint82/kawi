use kawi_buffer::{validate_text_edits, TextEdit};
use kawi_editor::Mode;
use kawi_plugin::{
    ApplyEditParams, ApplyEditResult, GetStateParams, GetStateResult, InvocationContext, Position,
    RpcError, Selection, UiMessageParams, INTERNAL_ERROR,
};
use ropey::Rope;
use serde_json::Value;

use super::super::App;

impl App {
    pub(crate) fn handle_plugin_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        match method {
            "editor.get_state" => self.plugin_get_state(params),
            "editor.apply_edit" => self.plugin_apply_edit(params),
            "ui.message" => self.plugin_message(params),
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

    fn plugin_message(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: UiMessageParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.show_message(params.message);
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

fn position_to_char(snapshot: &Rope, position: &Position) -> Result<usize, RpcError> {
    if position.line >= snapshot.len_lines() {
        return Err(RpcError::invalid_position(format!(
            "line {} is out of range",
            position.line
        )));
    }
    let line = snapshot.line(position.line);
    if position.byte_column > line.len_bytes() {
        return Err(RpcError::invalid_position(format!(
            "byte column {} is out of range",
            position.byte_column
        )));
    }
    let character = line.byte_to_char(position.byte_column);
    if line.char_to_byte(character) != position.byte_column {
        return Err(RpcError::invalid_position(format!(
            "byte column {} is not a UTF-8 boundary",
            position.byte_column
        )));
    }
    Ok(snapshot.line_to_char(position.line) + character)
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
