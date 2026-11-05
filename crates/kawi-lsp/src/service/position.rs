use super::CursorPosition;
use crate::text;
use lsp_types::{Position, PositionEncodingKind};
use ropey::Rope;

pub(super) fn cursor_position(
    text: &Rope,
    position: Position,
    encoding: &PositionEncodingKind,
) -> CursorPosition {
    let line = text::clamped_line(text, position.line);
    CursorPosition {
        line: u32::try_from(line).unwrap_or(u32::MAX),
        column: text::line_character(
            text,
            u32::try_from(line).unwrap_or(u32::MAX),
            position.character,
            encoding,
        ),
    }
}
