use super::*;
use kawi_lsp::{CompletionEntry, TextChange};
use std::path::PathBuf;

fn completion(request: u64) -> CompletionEvent {
    CompletionEvent {
        request,
        server: "fake".to_string(),
        path: PathBuf::from("/tmp/lsp_text.rs"),
        version: 1,
        original_hash: 0,
        cursor: 0,
        word_start: 0,
        prefix: String::new(),
        items: vec![CompletionEntry {
            server: "fake".to_string(),
            label: "value".to_string(),
            detail: None,
            filter_text: None,
            sort_text: None,
            documentation: None,
            primary: TextChange {
                start: 0,
                end: 0,
                text: "value".to_string(),
            },
            additional: Vec::new(),
            deprecated: false,
            preselect: false,
            snippet: false,
            resolve_id: None,
            command: None,
        }],
    }
}

#[test]
fn stale_completion_events_are_ignored() {
    let mut text = LspText {
        completion_request: Some(2),
        ..LspText::default()
    };
    assert!(!text.apply_completion(completion(1)));
    assert!(text.take_completion().is_none());
    assert!(text.apply_completion(completion(2)));
    assert!(text.take_completion().is_some());
}

#[test]
fn failures_clear_the_matching_request() {
    let mut text = LspText {
        code_action_request: Some(3),
        ..LspText::default()
    };
    assert_eq!(text.fail(3), Some(TextFailure::Report));
    assert_eq!(text.fail(3), None);
    assert!(text.take_code_actions().is_none());
}

#[test]
fn document_changes_keep_completion_resolve_alive() {
    let mut text = LspText {
        completion_request: Some(1),
        completion_resolve_request: Some(2),
        signature_request: Some(3),
        ..LspText::default()
    };
    assert!(text.cancel_document_requests(None));
    assert_eq!(text.completion_request, None);
    assert_eq!(text.signature_request, None);
    assert_eq!(text.completion_resolve_request, Some(2));
}

#[test]
fn automatic_completion_failures_are_silent() {
    let mut text = LspText {
        completion_request: Some(4),
        completion_report_errors: false,
        ..LspText::default()
    };
    assert_eq!(text.fail(4), Some(TextFailure::Silent));
}

#[test]
fn requests_without_a_service_report_unavailable() {
    let mut text = LspText::default();
    let result = text.completion(
        None,
        Path::new("/tmp/lsp_text.rs"),
        CursorPosition { line: 0, column: 0 },
        CompletionTrigger::invoked(),
    );
    assert_eq!(result, Err("lsp unavailable".to_string()));
}
