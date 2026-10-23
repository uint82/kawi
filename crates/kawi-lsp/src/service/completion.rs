use super::text_requests::{PendingTextRequest, TextResponse};
use super::*;
use crate::text;
use kawi_buffer::{content_hash, validate_text_edits};
use lsp_types::{CompletionResponse, CompletionTextEdit, Documentation, InsertTextFormat, Range};
use ropey::Rope;
use serde_json::Value;

const MAX_ITEMS: usize = 512;

pub(super) fn completion_event(
    pending: &PendingTextRequest,
    responses: &[TextResponse],
    text: &Rope,
) -> Result<CompletionEvent, String> {
    let position = text::position_from_column(
        text,
        pending.position.line,
        pending.position.column,
        &PositionEncodingKind::UTF32,
    );
    let cursor = text::char_offset(text, position, &PositionEncodingKind::UTF32)
        .ok_or_else(|| "Completion position is out of bounds".to_string())?;
    let prefix = identifier_prefix(text, cursor);
    let query = prefix.to_lowercase();
    let word_start = cursor.saturating_sub(prefix.chars().count());
    let mut candidates = Vec::new();
    let mut response_error = None;
    for response in responses {
        match completion_items(&response.value) {
            Ok(items) => {
                candidates.extend(items.into_iter().filter_map(|item| {
                    let score = match_score(&item, &prefix, &query)?;
                    Some(CompletionCandidate {
                        score,
                        sort_key: sort_key(&item),
                        server: response.server.clone(),
                        encoding: response.encoding.clone(),
                        item,
                    })
                }));
            }
            Err(error) => {
                response_error.get_or_insert(error);
            }
        }
    }
    if candidates.is_empty() {
        return Err(response_error.unwrap_or_else(|| "No completions".to_string()));
    }
    candidates.sort_by(|left, right| {
        left.score
            .cmp(&right.score)
            .then_with(|| left.sort_key.cmp(&right.sort_key))
    });
    let entries: Vec<CompletionEntry> = candidates
        .into_iter()
        .take(MAX_ITEMS)
        .filter_map(|candidate| {
            completion_entry(
                candidate.item,
                candidate.server,
                text,
                cursor,
                word_start,
                &candidate.encoding,
            )
        })
        .collect();
    if entries.is_empty() {
        return Err("No completions".to_string());
    }
    Ok(CompletionEvent {
        request: pending.request,
        server: responses
            .first()
            .map(|response| response.server.clone())
            .unwrap_or_default(),
        path: pending.path.clone(),
        version: pending.version,
        original_hash: content_hash(text.to_string().as_bytes()),
        items: entries,
    })
}

struct CompletionCandidate {
    score: u8,
    sort_key: String,
    server: String,
    item: lsp_types::CompletionItem,
    encoding: PositionEncodingKind,
}

fn completion_items(value: &Value) -> Result<Vec<lsp_types::CompletionItem>, String> {
    if value.is_null() {
        return Err("No completions".to_string());
    }
    let response = serde_json::from_value::<CompletionResponse>(value.clone())
        .map_err(|error| format!("Invalid completion response: {error}"))?;
    Ok(match response {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    })
}

fn identifier_prefix(text: &Rope, cursor: usize) -> String {
    let mut start = cursor;
    while start > 0 {
        let Some(character) = text.get_char(start - 1) else {
            break;
        };
        if !character.is_alphanumeric() && character != '_' {
            break;
        }
        start -= 1;
    }
    text.slice(start..cursor).chars().collect()
}

fn match_score(item: &lsp_types::CompletionItem, prefix: &str, query: &str) -> Option<u8> {
    if prefix.is_empty() {
        return Some(0);
    }
    let value = item.filter_text.as_deref().unwrap_or(&item.label);
    let lower = value.to_lowercase();
    if value.starts_with(prefix) {
        Some(0)
    } else if lower.starts_with(query) {
        Some(1)
    } else if is_subsequence(value, prefix) {
        Some(2)
    } else if is_subsequence(&lower, query) {
        Some(3)
    } else {
        None
    }
}

fn is_subsequence(value: &str, query: &str) -> bool {
    let mut query = query.chars();
    let Some(mut next) = query.next() else {
        return true;
    };
    for character in value.chars() {
        if character == next {
            let Some(following) = query.next() else {
                return true;
            };
            next = following;
        }
    }
    false
}

fn sort_key(item: &lsp_types::CompletionItem) -> String {
    item.sort_text.clone().unwrap_or_else(|| item.label.clone())
}

fn completion_entry(
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
    Some(CompletionEntry {
        server,
        label,
        detail: item.detail.as_deref().map(display_text),
        documentation: item.documentation.map(documentation_text),
        primary,
        additional,
        deprecated: item.deprecated.unwrap_or(false),
        preselect: item.preselect.unwrap_or(false),
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

fn completion_text(value: &str, format: InsertTextFormat) -> String {
    if format == InsertTextFormat::SNIPPET {
        snippet_plain(value)
    } else {
        value.to_string()
    }
}

fn convert_edit(
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

fn documentation_text(documentation: Documentation) -> String {
    match documentation {
        Documentation::String(value) => display_text(&value),
        Documentation::MarkupContent(markup) => display_text(&markup.value),
    }
}

fn display_text(value: &str) -> String {
    one_line_message(value)
}

fn snippet_plain(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '\\' if index + 1 < chars.len() => {
                output.push(chars[index + 1]);
                index += 2;
            }
            '$' => index = append_snippet_dollar(&chars, index, &mut output),
            character => {
                output.push(character);
                index += 1;
            }
        }
    }
    output
}

fn append_snippet_dollar(chars: &[char], index: usize, output: &mut String) -> usize {
    let Some(next) = chars.get(index + 1).copied() else {
        output.push('$');
        return index + 1;
    };
    if next.is_ascii_digit() {
        return skip_while(chars, index + 1, |character| character.is_ascii_digit());
    }
    if next == '{' {
        let Some(end) = matching_brace(chars, index + 1) else {
            return index + 1;
        };
        let body: String = chars[index + 2..end].iter().collect();
        append_placeholder(&body, output);
        return end + 1;
    }
    if next.is_alphanumeric() || next == '_' {
        return skip_while(chars, index + 1, |character| {
            character.is_alphanumeric() || character == '_'
        });
    }
    output.push('$');
    index + 1
}

fn append_placeholder(body: &str, output: &mut String) {
    let value = body.trim_start_matches(|character: char| character.is_ascii_digit());
    if let Some(choice) = value
        .strip_prefix('|')
        .and_then(|rest| rest.split('|').next())
    {
        if let Some(first) = choice.split(',').next() {
            output.push_str(&snippet_plain(first));
        }
        return;
    }
    if let Some((_, default)) = value.split_once(':') {
        output.push_str(&snippet_plain(default));
    }
}

fn matching_brace(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = start;
    while index < chars.len() {
        match chars[index] {
            '\\' => index = index.saturating_add(2),
            '{' => {
                depth = depth.saturating_add(1);
                index += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}

fn skip_while(chars: &[char], start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut index = start;
    while chars
        .get(index)
        .is_some_and(|character| predicate(*character))
    {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_common_snippet_placeholders() {
        assert_eq!(snippet_plain("println!(${1:value})$0"), "println!(value)");
        assert_eq!(snippet_plain("${1|one,two|} rest"), "one rest");
        assert_eq!(snippet_plain("$TM_FILENAME.ts"), ".ts");
        assert_eq!(snippet_plain("literal \\$1"), "literal $1");
    }

    #[test]
    fn keeps_plain_literal_dollars() {
        assert_eq!(snippet_plain("cost $"), "cost $");
        assert_eq!(snippet_plain("cost $$"), "cost $$");
    }

    #[test]
    fn sanitizes_control_characters_in_labels() {
        assert_eq!(display_text("value\u{1b}\tname"), "value name");
    }

    #[test]
    fn extracts_the_identifier_prefix_at_the_cursor() {
        let text = Rope::from_str("  value_1 rest");
        assert_eq!(identifier_prefix(&text, 9), "value_1");
        assert_eq!(identifier_prefix(&text, 2), "");
    }

    #[test]
    fn fallback_edits_replace_the_typed_prefix() {
        let text = Rope::from_str("val\n");
        let pending = PendingTextRequest {
            request: 1,
            kind: super::super::text_requests::TextRequestKind::Completion,
            path: std::path::PathBuf::from("/tmp/completion.rs"),
            position: CursorPosition { line: 0, column: 3 },
            version: 1,
            remaining: 1,
            responses: Vec::new(),
            error: None,
        };
        let responses = vec![TextResponse {
            server: "fake".to_string(),
            value: serde_json::json!([{ "label": "value" }]),
            encoding: PositionEncodingKind::UTF16,
        }];
        let event = completion_event(&pending, &responses, &text).expect("completion event");
        assert_eq!(event.items[0].primary.start, 0);
        assert_eq!(event.items[0].primary.end, 3);
        assert_eq!(event.items[0].primary.text, "value");
    }

    #[test]
    fn filters_items_by_filter_text_and_prefix() {
        let text = Rope::from_str("prin\n");
        let pending = PendingTextRequest {
            request: 1,
            kind: super::super::text_requests::TextRequestKind::Completion,
            path: std::path::PathBuf::from("/tmp/completion.rs"),
            position: CursorPosition { line: 0, column: 4 },
            version: 1,
            remaining: 1,
            responses: Vec::new(),
            error: None,
        };
        let responses = vec![TextResponse {
            server: "fake".to_string(),
            value: serde_json::json!([
                { "label": "println!", "filterText": "println" },
                { "label": "print!" },
                { "label": "unrelated" },
                { "label": "display", "filterText": "prin" }
            ]),
            encoding: PositionEncodingKind::UTF16,
        }];
        let event = completion_event(&pending, &responses, &text).expect("completion event");
        assert_eq!(
            event
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["display", "print!", "println!"]
        );
    }

    #[test]
    fn fuzzy_matches_rank_behind_prefix_matches() {
        assert_eq!(
            match_score(&completion_item("print!"), "prin", "prin"),
            Some(0)
        );
        assert_eq!(
            match_score(&completion_item("println!"), "pln", "pln"),
            Some(2)
        );
        assert_eq!(
            match_score(&completion_item("ValueError"), "val", "val"),
            Some(1)
        );
        assert_eq!(match_score(&completion_item("display"), "prn", "prn"), None);
        assert!(is_subsequence("println!", "pln"));
        assert!(!is_subsequence("println!", "pnx"));
    }

    fn completion_item(label: &str) -> lsp_types::CompletionItem {
        lsp_types::CompletionItem {
            label: label.to_string(),
            ..lsp_types::CompletionItem::default()
        }
    }
}
