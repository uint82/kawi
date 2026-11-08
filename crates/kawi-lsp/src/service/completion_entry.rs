use super::completion_edit::convert_edit;
use super::markup::markup_document;
use super::*;
use kawi_buffer::validate_text_edits;
use kawi_completion::Snippet;
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
    let (primary, snippet) = item_edit(&item, text, cursor, word_start, encoding)?;
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
    Some(CompletionEntry {
        server,
        label,
        kind: item.kind,
        filter_text: item.filter_text.as_deref().map(display_text),
        sort_text: item.sort_text.as_deref().map(display_text),
        detail: item.detail.as_deref().map(display_text),
        documentation: item.documentation.map(markup_document),
        primary,
        additional,
        deprecated: item.deprecated.unwrap_or(false),
        preselect: item.preselect.unwrap_or(false),
        snippet,
        resolve_id: None,
        command: item.command.map(super::convert_command),
    })
}

struct SnippetText {
    text: String,
    snippet: Option<Snippet>,
}

fn item_edit(
    item: &lsp_types::CompletionItem,
    text: &Rope,
    cursor: usize,
    word_start: usize,
    encoding: &PositionEncodingKind,
) -> Option<(TextChange, Option<Snippet>)> {
    let format = item
        .insert_text_format
        .unwrap_or(InsertTextFormat::PLAIN_TEXT);
    match item.text_edit.as_ref() {
        Some(CompletionTextEdit::Edit(edit)) => {
            let value = snippet_text(&edit.new_text, format);
            let change = convert_edit(text, &edit.range, &value.text, encoding)?;
            Some((change, value.snippet))
        }
        Some(CompletionTextEdit::InsertAndReplace(edit)) => {
            let value = snippet_text(&edit.new_text, format);
            let change = convert_edit(text, &edit.replace, &value.text, encoding)?;
            Some((change, value.snippet))
        }
        None => {
            let value = snippet_text(
                item.insert_text.as_deref().unwrap_or(item.label.as_str()),
                format,
            );
            Some((
                TextChange {
                    start: word_start,
                    end: cursor,
                    text: value.text,
                },
                value.snippet,
            ))
        }
    }
}

fn snippet_text(value: &str, format: InsertTextFormat) -> SnippetText {
    if format != InsertTextFormat::SNIPPET {
        return SnippetText {
            text: value.to_string(),
            snippet: None,
        };
    }
    let snippet = Snippet::parse(value);
    SnippetText {
        text: snippet.text().to_string(),
        snippet: Some(snippet),
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

    #[test]
    fn snippet_items_keep_tab_stops() {
        let text = Rope::from_str("");
        let item = lsp_types::CompletionItem {
            label: "println".to_string(),
            insert_text: Some("println!(${1:value})$0".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..lsp_types::CompletionItem::default()
        };
        let entry = completion_entry(
            item,
            "fake".to_string(),
            &text,
            0,
            0,
            &PositionEncodingKind::UTF16,
        )
        .expect("completion entry");
        assert_eq!(entry.primary.text, "println!(value)");
        let snippet = entry.snippet.expect("snippet");
        assert_eq!(snippet.stops()[0].range(), 9..14);
        assert_eq!(snippet.final_stop(), 15);
    }

    #[test]
    fn plain_items_have_no_snippet() {
        let text = Rope::from_str("");
        let item = lsp_types::CompletionItem {
            label: "value".to_string(),
            ..lsp_types::CompletionItem::default()
        };
        let entry = completion_entry(
            item,
            "fake".to_string(),
            &text,
            0,
            0,
            &PositionEncodingKind::UTF16,
        )
        .expect("completion entry");
        assert_eq!(entry.primary.text, "value");
        assert!(entry.snippet.is_none());
    }

    #[test]
    fn preserves_the_completion_kind() {
        let text = Rope::from_str("val\n");
        let item = lsp_types::CompletionItem {
            label: "value".to_string(),
            kind: Some(lsp_types::CompletionItemKind::VARIABLE),
            ..lsp_types::CompletionItem::default()
        };
        let entry = completion_entry(
            item,
            "fake".to_string(),
            &text,
            3,
            0,
            &PositionEncodingKind::UTF16,
        )
        .expect("completion entry");
        assert_eq!(entry.kind, Some(lsp_types::CompletionItemKind::VARIABLE));
    }

    #[test]
    fn missing_completion_kind_stays_empty() {
        let text = Rope::from_str("val\n");
        let item = lsp_types::CompletionItem {
            label: "value".to_string(),
            ..lsp_types::CompletionItem::default()
        };
        let entry = completion_entry(
            item,
            "fake".to_string(),
            &text,
            3,
            0,
            &PositionEncodingKind::UTF16,
        )
        .expect("completion entry");
        assert_eq!(entry.kind, None);
    }
}
