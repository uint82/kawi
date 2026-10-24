use super::completion_candidates::{
    compare_candidates, completion_candidates, completion_position, CompletionCandidate,
};
use super::completion_entry::completion_entry;
use super::text_requests::{PendingTextRequest, TextResponse};
use super::*;
use kawi_buffer::content_hash;
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
    let (mut candidates, response_error) = completion_candidates(responses, &prefix, &query);
    if candidates.is_empty() {
        return Err(response_error.unwrap_or_else(|| "No completions".to_string()));
    }
    candidates.sort_by(compare_candidates);
    let (items, resolve) = completion_entries(
        candidates.into_iter().take(MAX_ITEMS),
        text,
        cursor,
        word_start,
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
        },
        resolve,
    })
}

fn completion_entries(
    candidates: impl Iterator<Item = CompletionCandidate>,
    text: &Rope,
    cursor: usize,
    word_start: usize,
    next_resolve_id: &mut u64,
) -> (Vec<CompletionEntry>, Vec<ResolvedCandidate>) {
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
            key: ClientKey {
                server: "fake".to_string(),
                root: std::path::PathBuf::from("/tmp"),
            },
            value: serde_json::json!([{ "label": "value" }]),
            encoding: PositionEncodingKind::UTF16,
            resolve: false,
        }];
        let mut next_resolve_id = 0;
        let output = completion_event(&pending, &responses, &text, &mut next_resolve_id)
            .expect("completion event");
        assert_eq!(output.event.items[0].primary.start, 0);
        assert_eq!(output.event.items[0].primary.end, 3);
        assert_eq!(output.event.items[0].primary.text, "value");
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
            key: ClientKey {
                server: "fake".to_string(),
                root: std::path::PathBuf::from("/tmp"),
            },
            value: serde_json::json!([
                { "label": "println!", "filterText": "println" },
                { "label": "print!" },
                { "label": "unrelated" },
                { "label": "display", "filterText": "prin" }
            ]),
            encoding: PositionEncodingKind::UTF16,
            resolve: false,
        }];
        let mut next_resolve_id = 0;
        let output = completion_event(&pending, &responses, &text, &mut next_resolve_id)
            .expect("completion event");
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
}
