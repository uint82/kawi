use std::time::{Duration, Instant};

use ropey::Rope;

use super::*;
use crate::registry::registry;
use crate::span::SyntaxEdit;

fn edit_request(
    revision: u64,
    base_revision: u64,
    text: &str,
    edits: Vec<SyntaxEdit>,
) -> SyntaxRequest {
    SyntaxRequest {
        base_revision,
        edits,
        key: SyntaxKey {
            document: 1,
            revision,
            top: 0,
            end: text.lines().count(),
            language: LanguageId::Rust,
        },
        text: Rope::from_str(text),
    }
}

fn insertion(start: usize, text: &str) -> SyntaxEdit {
    SyntaxEdit {
        start,
        end: start,
        text: text.to_string(),
    }
}

fn request(text: &str, rows: usize) -> SyntaxRequest {
    SyntaxRequest {
        base_revision: 0,
        edits: Vec::new(),
        key: SyntaxKey {
            document: 1,
            revision: 1,
            top: 0,
            end: rows,
            language: LanguageId::Rust,
        },
        text: Rope::from_str(text),
    }
}

#[test]
fn service_notifies_the_wake_channel() {
    let (wake, wakes) = mpsc::channel();
    let service = SyntaxService::with_wake(wake);
    service.submit(request("fn main() {}\n", 1));
    wakes
        .recv_timeout(Duration::from_secs(5))
        .expect("wake notification");
    assert!(service.try_recv().is_some());
}

#[test]
fn service_returns_rust_spans() {
    let service = SyntaxService::new();
    service.submit(request("// hi\nfn main() {}\n", 2));
    let result = wait_for(&service).expect("result");
    assert_eq!(result.generation, 1);
    assert!(result
        .spans
        .iter()
        .any(|span| highlight_is(span, "comment")));
    assert!(result
        .spans
        .iter()
        .any(|span| highlight_is(span, "keyword.function")));
}

#[test]
fn synchronous_highlight_does_not_wait_for_the_worker() {
    let service = SyntaxService::new();
    let result = service.highlight_now(request("// hi\nfn main() {}\n", 2));
    assert!(result
        .spans
        .iter()
        .any(|span| highlight_is(span, "comment")));
    assert!(result
        .spans
        .iter()
        .any(|span| highlight_is(span, "keyword.function")));
    assert!(result.generation > 0);
}

#[test]
fn service_receives_a_result_with_a_timeout() {
    let service = SyntaxService::new();
    service.submit(request("fn main() {}\n", 1));
    let result = service
        .recv_timeout(Duration::from_secs(5))
        .expect("result");
    assert_eq!(result.key.revision, 1);
}

#[test]
fn service_reports_an_empty_timeout() {
    let service = SyntaxService::new();
    assert!(service.recv_timeout(Duration::ZERO).is_none());
}

#[test]
fn service_updates_after_an_edit() {
    let service = SyntaxService::new();
    service.submit(request("fn main() {}\n", 1));
    let first = wait_for(&service).expect("first result");
    assert!(first
        .spans
        .iter()
        .any(|span| highlight_is(span, "keyword.function")));

    let mut edited = request("/*\nfn main() {}\n", 2);
    edited.key.revision = 2;
    service.submit(edited);
    let second = wait_for(&service).expect("second result");
    assert!(second
        .spans
        .iter()
        .any(|span| highlight_is(span, "comment") && span.row == 1));
}

#[test]
fn service_applies_incremental_edits() {
    let service = SyntaxService::new();
    service.submit(edit_request(1, 1, "fn main() {}\n", Vec::new()));
    wait_for(&service).expect("first result");

    let edited = "// note\nfn main() {}\n";
    service.submit(edit_request(2, 1, edited, vec![insertion(0, "// note\n")]));
    let second = wait_for(&service).expect("second result");
    assert!(second
        .spans
        .iter()
        .any(|span| highlight_is(span, "comment") && span.row == 0));
}

#[test]
fn queued_requests_merge_when_edits_chain() {
    let mut queued = Queued {
        generation: 1,
        request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
    };
    let next = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
    assert!(can_merge(&queued, &next));
    merge_request(&mut queued, 2, next);
    assert_eq!(queued.generation, 2);
    assert_eq!(queued.request.base_revision, 0);
    assert_eq!(queued.request.key.revision, 2);
    assert_eq!(queued.request.edits.len(), 2);
    assert_eq!(queued.request.text.to_string(), "ab");
}

#[test]
fn queued_requests_without_edits_do_not_merge() {
    let queued = Queued {
        generation: 1,
        request: edit_request(1, 0, "a", Vec::new()),
    };
    let next = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
    assert!(!can_merge(&queued, &next));
}

#[test]
fn queued_requests_do_not_merge_across_a_base_gap() {
    let queued = Queued {
        generation: 1,
        request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
    };
    let next = edit_request(2, 2, "ab", vec![insertion(1, "b")]);
    assert!(!can_merge(&queued, &next));
}

#[test]
fn queued_requests_do_not_merge_across_documents_or_languages() {
    let queued = Queued {
        generation: 1,
        request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
    };
    let mut other_document = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
    other_document.key.document = 2;
    assert!(!can_merge(&queued, &other_document));

    let mut other_language = edit_request(2, 1, "ab", vec![insertion(1, "b")]);
    other_language.key.language = LanguageId::Python;
    assert!(!can_merge(&queued, &other_language));
}

#[test]
fn queued_requests_merge_viewport_only_updates() {
    let mut queued = Queued {
        generation: 1,
        request: edit_request(1, 0, "a", vec![insertion(0, "a")]),
    };
    let next = edit_request(1, 1, "a", Vec::new());
    assert!(can_merge(&queued, &next));
    merge_request(&mut queued, 2, next);
    assert_eq!(queued.request.edits.len(), 1);
}

fn highlight_is(span: &SyntaxSpan, name: &str) -> bool {
    registry().name(span.highlight) == Some(name)
}

fn wait_for(service: &SyntaxService) -> Option<SyntaxResult> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(result) = service.try_recv() {
            return Some(result);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}
