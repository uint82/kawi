use super::text_requests::{PendingTextRequest, TextResponse};
use super::*;
use crate::text;
use lsp_types::CompletionResponse;
use ropey::Rope;
use serde_json::Value;
use std::cmp::Ordering;

pub(super) struct CompletionCandidate {
    pub(super) score: u8,
    pub(super) sort_key: String,
    pub(super) server: String,
    pub(super) key: ClientKey,
    pub(super) item: lsp_types::CompletionItem,
    pub(super) encoding: PositionEncodingKind,
    pub(super) resolve: bool,
}

pub(super) fn completion_position(
    pending: &PendingTextRequest,
    text: &Rope,
) -> Result<(usize, String, usize), String> {
    let position = text::position_from_column(
        text,
        pending.position.line,
        pending.position.column,
        &PositionEncodingKind::UTF32,
    );
    let cursor = text::char_offset(text, position, &PositionEncodingKind::UTF32)
        .ok_or_else(|| "Completion position is out of bounds".to_string())?;
    let prefix = identifier_prefix(text, cursor);
    let word_start = cursor.saturating_sub(prefix.chars().count());
    Ok((cursor, prefix, word_start))
}

pub(super) fn completion_candidates(
    responses: &[TextResponse],
    prefix: &str,
    query: &str,
) -> (Vec<CompletionCandidate>, Option<String>) {
    let mut candidates = Vec::new();
    let mut response_error = None;
    for response in responses {
        match completion_items(&response.value) {
            Ok(items) => candidates.extend(
                items
                    .into_iter()
                    .filter_map(|item| completion_candidate(item, response, prefix, query)),
            ),
            Err(error) => {
                response_error.get_or_insert(error);
            }
        }
    }
    (candidates, response_error)
}

fn completion_candidate(
    item: lsp_types::CompletionItem,
    response: &TextResponse,
    prefix: &str,
    query: &str,
) -> Option<CompletionCandidate> {
    let score = match_score(&item, prefix, query)?;
    Some(CompletionCandidate {
        score,
        sort_key: sort_key(&item),
        server: response.server.clone(),
        key: response.key.clone(),
        encoding: response.encoding.clone(),
        resolve: response.resolve,
        item,
    })
}

pub(super) fn compare_candidates(
    left: &CompletionCandidate,
    right: &CompletionCandidate,
) -> Ordering {
    left.score
        .cmp(&right.score)
        .then_with(|| left.sort_key.cmp(&right.sort_key))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_identifier_prefix_at_the_cursor() {
        let text = Rope::from_str("  value_1 rest");
        assert_eq!(identifier_prefix(&text, 9), "value_1");
        assert_eq!(identifier_prefix(&text, 2), "");
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
