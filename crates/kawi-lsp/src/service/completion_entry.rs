use super::completion_edit::convert_edit;
use super::completion_snippet::completion_text;
use super::markup::markup_document;
use super::*;
use kawi_buffer::validate_text_edits;
use lsp_types::{CompletionTextEdit, InsertTextFormat};
use ropey::Rope;

pub(super) fn completion_entry(
    item: lsp_types::CompletionItem,
    server: String,
    text: &Rope,
    cursor: usize,
    word_start: usize,
    encoding: &PositionEncodingKind,
) -> Option<CompletionEntry> {
    let primary = item_edit(&item, text, cursor, word_start, encoding)?;
    let additional = item
        .additional_text_edits
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|edit| convert_edit(text, &edit.range, &edit.new_text, encoding))
        .collect::<Option<Vec<_>>>()?;
    let mut edits = Vec::with_capacity(additional.len().saturating_add(1));
    edits.push(primary.clone());
    edits.extend(additional.iter().cloned());
    validate_text_edits(text.len_chars(), &edits).ok()?;
    let label = display_text(&item.label);
    if label.is_empty() {
        return None;
    }
    let format = item
        .insert_text_format
        .unwrap_or(InsertTextFormat::PLAIN_TEXT);
    Some(CompletionEntry {
        server,
        label,
        filter_text: item.filter_text.as_deref().map(display_text),
        sort_text: item.sort_text.as_deref().map(display_text),
        detail: item.detail.as_deref().map(display_text),
        documentation: item.documentation.map(markup_document),
        primary,
        additional,
        deprecated: item.deprecated.unwrap_or(false),
        preselect: item.preselect.unwrap_or(false),
        snippet: format == InsertTextFormat::SNIPPET,
        resolve_id: None,
        command: item.command.map(super::convert_command),
    })
}

fn item_edit(
    item: &lsp_types::CompletionItem,
    text: &Rope,
    cursor: usize,
    word_start: usize,
    encoding: &PositionEncodingKind,
) -> Option<TextChange> {
    let format = item
        .insert_text_format
        .unwrap_or(InsertTextFormat::PLAIN_TEXT);
    match item.text_edit.as_ref() {
        Some(CompletionTextEdit::Edit(edit)) => convert_edit(
            text,
            &edit.range,
            &completion_text(&edit.new_text, format),
            encoding,
        ),
        Some(CompletionTextEdit::InsertAndReplace(edit)) => convert_edit(
            text,
            &edit.replace,
            &completion_text(&edit.new_text, format),
            encoding,
        ),
        None => {
            let value = item.insert_text.as_deref().unwrap_or(item.label.as_str());
            Some(TextChange {
                start: word_start,
                end: cursor,
                text: completion_text(value, format),
            })
        }
    }
}

fn display_text(value: &str) -> String {
    one_line_message(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_control_characters_in_labels() {
        assert_eq!(display_text("value\u{1b}\tname"), "value name");
    }
}
