#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Prefix {
    pub(super) text: String,
    pub(super) start: usize,
    pub(super) cursor: usize,
}

pub(super) fn completion_prefix(editor: &kawi_editor::Editor) -> Prefix {
    let text = editor.document_snapshot();
    let row = editor.row().min(text.len_lines().saturating_sub(1));
    let line_start = text.line_to_char(row);
    let line_end = line_start.saturating_add(text.line(row).len_chars());
    let cursor = line_start.saturating_add(editor.col()).min(line_end);
    let mut start = cursor;
    while start > line_start {
        let Some(character) = text.get_char(start - 1) else {
            break;
        };
        if !character.is_alphanumeric() && character != '_' {
            break;
        }
        start -= 1;
    }
    Prefix {
        text: text.slice(start..cursor).chars().collect(),
        start,
        cursor,
    }
}
