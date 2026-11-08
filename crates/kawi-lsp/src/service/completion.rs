use super::completion_candidates::{
    compare_candidates, completion_candidates, completion_position, CompletionCandidate,
};
use super::completion_entry::completion_entry;
use super::text_requests::{PendingTextRequest, TextResponse};
use super::*;
use kawi_buffer::content_hash;
use kawi_completion::SnippetContext;
use ropey::Rope;

const MAX_ITEMS: usize = 512;

pub(super) struct CompletionOutput {
    pub(super) event: CompletionEvent,
    pub(super) resolve: Vec<ResolvedCandidate>,
}

pub(super) struct ResolvedCandidate {
    pub(super) id: u64,
    pub(super) key: ClientKey,
    pub(super) item: lsp_types::CompletionItem,
}

pub(super) fn completion_event(
    pending: &PendingTextRequest,
    responses: &[TextResponse],
    text: &Rope,
    next_resolve_id: &mut u64,
) -> Result<CompletionOutput, String> {
    let (cursor, prefix, word_start) = completion_position(pending, text)?;
    let query = prefix.to_lowercase();
    let mut candidates = completion_candidates(responses, &prefix, &query);
    if candidates.items.is_empty() {
        return Err(candidates
            .error
            .unwrap_or_else(|| "No completions".to_string()));
    }
    let truncated = candidates.items.len() > MAX_ITEMS;
    let is_incomplete = candidates.is_incomplete || truncated;
    candidates.items.sort_by(compare_candidates);
    let (items, resolve) = completion_entries(
        candidates.items.into_iter().take(MAX_ITEMS),
        text,
        cursor,
        word_start,
        &pending.path,
        next_resolve_id,
    );
    if items.is_empty() {
        return Err("No completions".to_string());
    }
    Ok(CompletionOutput {
        event: CompletionEvent {
            request: pending.request,
            server: responses
                .first()
                .map(|response| response.server.clone())
                .unwrap_or_default(),
            path: pending.path.clone(),
            version: pending.version,
            original_hash: content_hash(text.to_string().as_bytes()),
            cursor,
            word_start,
            prefix,
            items,
            is_incomplete,
        },
        resolve,
    })
}

fn completion_entries(
    candidates: impl Iterator<Item = CompletionCandidate>,
    text: &Rope,
    cursor: usize,
    word_start: usize,
    path: &Path,
    next_resolve_id: &mut u64,
) -> (Vec<CompletionEntry>, Vec<ResolvedCandidate>) {
    let source = text.to_string();
    let context = SnippetContext::new(Some(path), &source, cursor);
    let mut entries = Vec::new();
    let mut resolve = Vec::new();
    for candidate in candidates {
        let raw = candidate.resolve.then(|| candidate.item.clone());
        let key = candidate.key;
        let Some(mut entry) = completion_entry(
            candidate.item,
            candidate.server,
            text,
            cursor,
            word_start,
            &candidate.encoding,
            &context,
        ) else {
            continue;
        };
        if let Some(item) = raw {
            let id = allocate_resolve_id(next_resolve_id);
            entry.resolve_id = Some(id);
            resolve.push(ResolvedCandidate { id, key, item });
        }
        entries.push(entry);
    }
    (entries, resolve)
}

fn allocate_resolve_id(next: &mut u64) -> u64 {
    let id = *next;
    *next = next.saturating_add(1);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(column: u32) -> PendingTextRequest {
        PendingTextRequest {
            request: 1,
            kind: super::super::text_requests::TextRequestKind::Completion,
            path: std::path::PathBuf::from("/tmp/completion.rs"),
            position: CursorPosition { line: 0, column },
            version: 1,
            remaining: 1,
            responses: Vec::new(),
            error: None,
        }
    }

    fn response(value: serde_json::Value) -> TextResponse {
        TextResponse {
            server: "fake".to_string(),
            key: ClientKey {
                server: "fake".to_string(),
                root: std::path::PathBuf::from("/tmp"),
            },
            value,
            encoding: PositionEncodingKind::UTF16,
            resolve: false,
        }
    }

    fn completion(text: &str, column: u32, value: serde_json::Value) -> CompletionOutput {
        completion_with(text, column, vec![response(value)])
    }

    fn completion_with(text: &str, column: u32, responses: Vec<TextResponse>) -> CompletionOutput {
        let mut next_resolve_id = 0;
        completion_event(
            &request(column),
            &responses,
            &Rope::from_str(text),
            &mut next_resolve_id,
        )
        .expect("completion event")
    }

    #[test]
    fn fallback_edits_replace_the_typed_prefix() {
        let output = completion("val\n", 3, serde_json::json!([{ "label": "value" }]));
        assert!(!output.event.is_incomplete);
        assert_eq!(output.event.items[0].primary.start, 0);
        assert_eq!(output.event.items[0].primary.end, 3);
        assert_eq!(output.event.items[0].primary.text, "value");
    }

    #[test]
    fn completion_items_expand_context_variables() {
        let output = completion(
            "val\n",
            3,
            serde_json::json!({
                "isIncomplete": false,
                "itemDefaults": { "insertTextFormat": 2 },
                "items": [{ "label": "val", "insertText": "$TM_FILENAME" }]
            }),
        );
        assert_eq!(output.event.items[0].primary.text, "completion.rs");
    }

    #[test]
    fn filters_items_by_filter_text_and_prefix() {
        let output = completion(
            "prin\n",
            4,
            serde_json::json!([
                { "label": "println!", "filterText": "println" },
                { "label": "print!" },
                { "label": "unrelated" },
                { "label": "display", "filterText": "prin" }
            ]),
        );
        assert_eq!(
            output
                .event
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["display", "print!", "println!"]
        );
    }

    #[test]
    fn incomplete_completion_lists_are_preserved() {
        let output = completion(
            "val\n",
            3,
            serde_json::json!({
                "isIncomplete": true,
                "items": [{ "label": "value" }]
            }),
        );
        assert!(output.event.is_incomplete);
    }

    #[test]
    fn any_incomplete_response_marks_the_event_incomplete() {
        let output = completion_with(
            "val\n",
            3,
            vec![
                response(serde_json::json!({
                    "isIncomplete": false,
                    "items": [{ "label": "value" }]
                })),
                response(serde_json::json!({
                    "isIncomplete": true,
                    "items": [{ "label": "other" }]
                })),
            ],
        );
        assert!(output.event.is_incomplete);
    }

    #[test]
    fn truncated_completion_lists_are_incomplete() {
        let items: Vec<_> = (0..=MAX_ITEMS)
            .map(|index| serde_json::json!({ "label": format!("item{index}") }))
            .collect();
        let output = completion(
            "\n",
            0,
            serde_json::json!({
                "isIncomplete": false,
                "items": items
            }),
        );
        assert_eq!(output.event.items.len(), MAX_ITEMS);
        assert!(output.event.is_incomplete);
    }
}
