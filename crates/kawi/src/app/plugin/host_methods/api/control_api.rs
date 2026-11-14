use kawi_editor::Mode;
use kawi_plugin::{RpcError, Selection, SetCursorParams, SetModeParams, SetSelectionParams};
use serde_json::Value;

use super::super::super::super::App;
use super::super::invalid_params;
use super::ensure_active_buffer;

impl App {
    pub(super) fn plugin_set_cursor(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: SetCursorParams = serde_json::from_value(params).map_err(invalid_params)?;
        ensure_active_buffer(self, params.buffer)?;
        self.editor
            .set_cursor_position(params.cursor.line, params.cursor.byte_column);
        self.refresh_after_control_change();
        super::super::json_result(self.plugin_state(false))
    }

    pub(super) fn plugin_set_selection(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: SetSelectionParams = serde_json::from_value(params).map_err(invalid_params)?;
        ensure_active_buffer(self, params.buffer)?;
        match params.selection {
            Some(selection) => {
                let snapshot = self.editor.document_snapshot();
                let (start, end) = selection_ends(&snapshot, &selection)?;
                self.editor.set_selection_ends(start, end);
            }
            None => {
                self.editor.clear_visual_selection();
            }
        }
        self.refresh_after_control_change();
        super::super::json_result(self.plugin_state(false))
    }

    pub(super) fn plugin_set_mode(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: SetModeParams = serde_json::from_value(params).map_err(invalid_params)?;
        ensure_active_buffer(self, params.buffer)?;
        let Some(mode) = parse_mode(&params.mode) else {
            return Err(RpcError::mode_not_supported(&params.mode));
        };
        if !self.editor.set_editor_mode(mode) {
            return Err(RpcError::mode_not_supported(&params.mode));
        }
        self.refresh_after_control_change();
        super::super::json_result(self.plugin_state(false))
    }

    fn refresh_after_control_change(&mut self) {
        self.close_completion_on_mode_change();
        self.lsp.sync_editor(&self.editor);
        self.reveal_cursor();
    }
}

fn parse_mode(name: &str) -> Option<Mode> {
    match name {
        "normal" => Some(Mode::Normal),
        "insert" => Some(Mode::Insert),
        "replace" => Some(Mode::Replace),
        "visual" => Some(Mode::Visual),
        "visual_line" => Some(Mode::VisualLine),
        "visual_block" => Some(Mode::VisualBlock),
        _ => None,
    }
}

type CharSelection = ((usize, usize), (usize, usize));

fn selection_ends(
    snapshot: &ropey::Rope,
    selection: &Selection,
) -> Result<CharSelection, RpcError> {
    Ok((
        char_position(snapshot, &selection.start)?,
        char_position(snapshot, &selection.end)?,
    ))
}

fn char_position(
    snapshot: &ropey::Rope,
    position: &kawi_plugin::Position,
) -> Result<(usize, usize), RpcError> {
    let offset = super::super::position_to_char(snapshot, position)?;
    let line = snapshot.char_to_line(offset);
    Ok((line, offset.saturating_sub(snapshot.line_to_char(line))))
}
