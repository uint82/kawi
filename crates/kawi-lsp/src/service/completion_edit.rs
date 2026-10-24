use super::*;
use crate::text;
use lsp_types::Range;
use ropey::Rope;

pub(super) fn convert_edit(
    text: &Rope,
    range: &Range,
    value: &str,
    encoding: &PositionEncodingKind,
) -> Option<TextChange> {
    let start = text::char_offset(text, range.start, encoding)?;
    let end = text::char_offset(text, range.end, encoding)?;
    (start <= end).then(|| TextChange {
        start,
        end,
        text: value.to_string(),
    })
}
