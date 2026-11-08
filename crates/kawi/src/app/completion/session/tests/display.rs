use super::*;
use kawi_completion::Snippet;

#[test]
fn snippet_items_are_marked() {
    let mut item = item("a", "println!");
    item.snippet = Some(Snippet::parse("println!($1)"));
    let session = CompletionSession::new(event(vec![item]), "");
    assert_eq!(labels(&session), ["println!"]);
    assert_eq!(session.items()[0].kind, Some(CompletionKind::Snippet));
}

#[test]
fn explicit_kinds_are_preserved_and_measured() {
    let mut function = item("a", "print");
    function.kind = Some(CompletionKind::Function);
    let mut property = item("b", "width");
    property.kind = Some(CompletionKind::Property);
    let session = CompletionSession::new(event(vec![function, property]), "");
    assert_eq!(session.items()[0].kind, Some(CompletionKind::Function));
    assert_eq!(session.items()[1].kind, Some(CompletionKind::Property));
    assert_eq!(session.kind_width(), CompletionKind::Function.name().len());
    assert_eq!(session.list_width(), 5 + 2 + 8 + 2);
}

#[test]
fn missing_kinds_leave_the_kind_column_empty() {
    let session = CompletionSession::new(event(vec![item("a", "value")]), "");
    assert_eq!(session.items()[0].kind, None);
    assert_eq!(session.kind_width(), 0);
    assert_eq!(session.list_width(), 5 + 2);
}

#[test]
fn text_kinds_use_the_snippet_kind_when_snippeting() {
    let mut value = item("a", "println!");
    value.kind = Some(CompletionKind::Text);
    value.snippet = Some(Snippet::parse("println!($1)"));
    let session = CompletionSession::new(event(vec![value]), "");
    assert_eq!(session.items()[0].kind, Some(CompletionKind::Snippet));
}

#[test]
fn deprecated_items_stay_marked() {
    let mut value = item("a", "value");
    value.deprecated = true;
    let session = CompletionSession::new(event(vec![value]), "");
    assert!(session.items()[0].deprecated);
}

#[test]
fn fuzzy_matches_highlight_the_label() {
    let session = CompletionSession::new(event(vec![item("a", "print")]), "pnt");
    assert_eq!(session.items()[0].highlights, [0..1, 3..5]);
}

#[test]
fn empty_queries_do_not_highlight_labels() {
    let session = CompletionSession::new(event(vec![item("a", "print")]), "");
    assert!(session.items()[0].highlights.is_empty());
}

#[test]
fn highlights_keep_byte_offsets_of_multibyte_labels() {
    let session = CompletionSession::new(event(vec![item("a", "école")]), "ol");
    let highlights = &session.items()[0].highlights;
    assert_eq!(highlights.len(), 1);
    assert_eq!(highlights[0], 3..5);
}

#[test]
fn filter_text_matches_are_mapped_back_to_the_label() {
    let mut value = item("a", "value");
    value.filter_text = Some("l".to_string());
    let session = CompletionSession::new(event(vec![value]), "l");
    let highlights = &session.items()[0].highlights;
    assert_eq!(highlights.len(), 1);
    assert_eq!(highlights[0], 2..3);
}

#[test]
fn items_filtered_by_filter_text_without_a_label_match_have_no_highlights() {
    let mut value = item("a", "value");
    value.filter_text = Some("value()".to_string());
    let session = CompletionSession::new(event(vec![value]), "()");
    assert!(session.items()[0].highlights.is_empty());
}

#[test]
fn resolved_items_keep_their_highlights() {
    let mut value = item("a", "value");
    value.resolve = true;
    let mut session = CompletionSession::new(event(vec![value]), "val");
    let item = session.needs_resolve().expect("resolve").clone();
    session.start_resolve(7, &item);
    assert!(session.apply_resolved(ResolvedItem {
        request: 7,
        item_id: "a".to_string(),
        detail: Some("detail".to_string()),
        documentation: None,
        additional: Vec::new(),
    }));
    let highlights = &session.items()[0].highlights;
    assert_eq!(highlights.len(), 1);
    assert_eq!(highlights[0], 0..3);
}

#[test]
fn resolved_detail_keeps_the_list_width() {
    let mut value = item("a", "value");
    value.resolve = true;
    let mut session = CompletionSession::new(event(vec![value]), "");
    let width = session.list_width();
    let item = session.needs_resolve().expect("resolve").clone();
    session.start_resolve(7, &item);
    assert!(session.apply_resolved(ResolvedItem {
        request: 7,
        item_id: "a".to_string(),
        detail: Some("a much longer detail".to_string()),
        documentation: None,
        additional: Vec::new(),
    }));
    assert_eq!(session.list_width(), width);
    assert!(session.items()[0]
        .detail
        .as_deref()
        .unwrap_or_default()
        .contains("a much longer detail"));
}
