use super::completion_edit::convert_edit;
use super::markup::markup_document;
use super::*;
use kawi_buffer::validate_text_edits;
use kawi_completion::{Snippet, SnippetContext};
use lsp_types::{CompletionTextEdit, InsertTextFormat, InsertTextMode};
use ropey::Rope;

pub(super) fn completion_entry(
    item: lsp_types::CompletionItem,
    server: String,
    text: &Rope,
    cursor: usize,
    word_start: usize,
    encoding: &PositionEncodingKind,
    context: &SnippetContext,
) -> Option<CompletionEntry> {
    let (primary, snippet) = item_edit(&item, text, cursor, word_start, encoding, context)?;
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
    context: &SnippetContext,
) -> Option<(TextChange, Option<Snippet>)> {
    let format = item
        .insert_text_format
        .unwrap_or(InsertTextFormat::PLAIN_TEXT);
    let mode = item.insert_text_mode.unwrap_or(InsertTextMode::AS_IS);
    match item.text_edit.as_ref() {
        Some(CompletionTextEdit::Edit(edit)) => {
            let value = snippet_text(&edit.new_text, format, mode, context);
            let change = convert_edit(text, &edit.range, &value.text, encoding)?;
            Some((change, value.snippet))
        }
        Some(CompletionTextEdit::InsertAndReplace(edit)) => {
            let value = snippet_text(&edit.new_text, format, mode, context);
            let change = convert_edit(text, &edit.replace, &value.text, encoding)?;
            Some((change, value.snippet))
        }
        None => {
            let value = snippet_text(
                item.insert_text.as_deref().unwrap_or(item.label.as_str()),
                format,
                mode,
                context,
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

fn snippet_text(
    value: &str,
    format: InsertTextFormat,
    mode: InsertTextMode,
    context: &SnippetContext,
) -> SnippetText {
    let value = adjusted_text(value, mode, context);
    if format != InsertTextFormat::SNIPPET {
        return SnippetText {
            text: value,
            snippet: None,
        };
    }
    let snippet = Snippet::parse_with_context(&value, context);
    SnippetText {
        text: snippet.text().to_string(),
        snippet: Some(snippet),
    }
}

fn adjusted_text(value: &str, mode: InsertTextMode, context: &SnippetContext) -> String {
    if mode != InsertTextMode::ADJUST_INDENTATION {
        return value.to_string();
    }
    let mut lines = value.split('\n');
    let Some(first) = lines.next() else {
        return String::new();
    };
    let rest: Vec<&str> = lines.collect();
    let common = rest
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.len()
                .saturating_sub(line.trim_start_matches([' ', '\t']).len())
        })
        .min()
        .unwrap_or_default();
    let mut adjusted = String::from(first);
    for line in rest {
        adjusted.push('\n');
        if line.trim().is_empty() {
            continue;
        }
        let line = &line[common.min(line.len())..];
        adjusted.push_str(context.line_indent());
        adjusted.push_str(line);
    }
    adjusted
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
            &SnippetContext::default(),
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
            &SnippetContext::default(),
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
            &SnippetContext::default(),
        )
        .expect("completion entry");
        assert_eq!(entry.kind, Some(lsp_types::CompletionItemKind::VARIABLE));
    }

    #[test]
    fn adjust_indentation_matches_the_current_line() {
        let text = Rope::from_str("  value");
        let context = SnippetContext::new(None, "  value", 2);
        let item = lsp_types::CompletionItem {
            label: "value".to_string(),
            insert_text: Some("one\n    two\n      three".to_string()),
            insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
            ..lsp_types::CompletionItem::default()
        };
        let entry = completion_entry(
            item,
            "fake".to_string(),
            &text,
            2,
            2,
            &PositionEncodingKind::UTF16,
            &context,
        )
        .expect("completion entry");
        assert_eq!(entry.primary.text, "one\n  two\n    three");
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
            &SnippetContext::default(),
        )
        .expect("completion entry");
        assert_eq!(entry.kind, None);
    }
}
