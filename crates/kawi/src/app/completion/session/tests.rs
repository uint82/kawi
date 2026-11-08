use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_completion::{CompletionKind, MarkupKind, ResolvedItem};
use std::path::PathBuf;

fn edit(start: usize, end: usize, text: &str) -> TextEdit {
    TextEdit {
        start,
        end,
        text: text.to_string(),
    }
}

fn item(id: &str, label: &str) -> Item {
    Item {
        source: "test".to_string(),
        id: id.to_string(),
        label: label.to_string(),
        kind: None,
        filter_text: None,
        sort_text: None,
        detail: None,
        documentation: None,
        deprecated: false,
        preselect: false,
        snippet: None,
        resolve: false,
        primary: edit(0, 0, label),
        additional: Vec::new(),
        command: None,
    }
}

fn labels(session: &CompletionSession) -> Vec<&str> {
    session
        .items()
        .iter()
        .map(|line| line.label.as_str())
        .collect()
}

fn event(items: Vec<Item>) -> Event {
    Event {
        request: 1,
        path: PathBuf::from("/tmp/session.rs"),
        version: 1,
        original_hash: 0,
        cursor: 0,
        word_start: 0,
        prefix: String::new(),
        items,
        is_incomplete: false,
    }
}

#[test]
fn filters_and_preselects_the_server_choice() {
    let mut second = item("second", "print_debug");
    second.preselect = true;
    let session = CompletionSession::new(
        event(vec![item("first", "print"), second, item("third", "value")]),
        "pr",
    );
    assert_eq!(labels(&session), ["print", "print_debug"]);
    assert_eq!(
        session.selected_item().map(|item| item.id.as_str()),
        Some("second")
    );
}

#[test]
fn keeps_the_selected_item_when_the_event_refreshes() {
    let mut session =
        CompletionSession::new(event(vec![item("a", "alpha"), item("b", "beta")]), "");
    session.handle_key(
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        None,
    );
    assert_eq!(
        session.selected_item().map(|item| item.id.as_str()),
        Some("b")
    );
    session.update(event(vec![item("a", "alpha"), item("b", "beta")]), "");
    assert_eq!(
        session.selected_item().map(|item| item.id.as_str()),
        Some("b")
    );
}

#[test]
fn adjusted_edits_follow_later_typing() {
    let session = CompletionSession::new(
        Event {
            cursor: 2,
            word_start: 0,
            items: vec![Item {
                primary: edit(0, 2, "value"),
                ..item("a", "value")
            }],
            ..event(Vec::new())
        },
        "",
    );
    let edit = session.workspace_edit(4).expect("edit").documents[0].edits[0].clone();
    assert_eq!(edit.end, 4);
}

#[test]
fn later_additional_edits_follow_typing() {
    let mut item = item("a", "value");
    item.primary = edit(0, 2, "value");
    item.additional = vec![edit(4, 4, "// note\n")];
    let session = CompletionSession::new(
        Event {
            cursor: 2,
            word_start: 0,
            items: vec![item],
            ..event(Vec::new())
        },
        "",
    );
    let edit = session.workspace_edit(4).expect("edit").documents[0].edits[1].clone();
    assert_eq!(edit.start, 6);
    assert_eq!(edit.end, 6);
}

#[test]
fn resolve_requests_are_deduplicated() {
    let mut item = item("a", "value");
    item.resolve = true;
    let mut session = CompletionSession::new(event(vec![item]), "");
    let item = session.needs_resolve().expect("resolve").clone();
    session.start_resolve(7, &item);
    assert!(session.needs_resolve().is_none());
}

#[test]
fn failed_resolve_is_not_retried() {
    let mut item = item("a", "value");
    item.resolve = true;
    let mut session = CompletionSession::new(event(vec![item]), "");
    let item = session.needs_resolve().expect("resolve").clone();
    session.start_resolve(7, &item);
    assert!(session.resolve_failed(7));
    assert!(session.needs_resolve().is_none());
}

#[test]
fn resolved_items_without_documentation_are_not_retried() {
    let mut value = item("a", "value");
    value.resolve = true;
    let mut session = CompletionSession::new(event(vec![value]), "");
    let value = session.needs_resolve().expect("resolve").clone();
    session.start_resolve(7, &value);
    assert!(session.apply_resolved(ResolvedItem {
        request: 7,
        item_id: "a".to_string(),
        detail: Some("resolved".to_string()),
        documentation: None,
        additional: Vec::new(),
    }));
    assert!(session.needs_resolve().is_none());
}

#[test]
fn docs_width_grows_to_the_largest_seen() {
    let mut session = CompletionSession::new(event(Vec::new()), "");
    session.grow_docs_width(30);
    session.grow_docs_width(20);
    assert_eq!(session.docs_width(), 30);
}

#[test]
fn docs_side_is_fixed_for_the_session() {
    let mut session = CompletionSession::new(event(vec![item("a", "value")]), "");
    session.set_docs_side(DocsSide::Right);
    session.update(event(vec![item("a", "value")]), "");
    assert_eq!(session.docs_side(), Some(DocsSide::Right));
}

#[test]
fn empty_documentation_is_hidden() {
    let mut value = item("a", "value");
    value.documentation = Some(Documentation {
        text: " \n".to_string(),
        kind: MarkupKind::PlainText,
    });
    let session = CompletionSession::new(event(vec![value]), "");
    assert!(session.selected_document().is_none());
}

#[test]
fn docs_scroll_uses_page_and_limit() {
    let mut value = item("a", "value");
    value.documentation = Some(Documentation {
        text: "one\ntwo\nthree\nfour\nfive".to_string(),
        kind: MarkupKind::PlainText,
    });
    let mut session = CompletionSession::new(event(vec![value]), "");
    let down = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);
    session.handle_key(
        down,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        down,
        Some((3, 2)),
    );
    assert_eq!(session.docs_scroll(), 2);
    session.handle_key(
        down,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        down,
        Some((3, 2)),
    );
    assert_eq!(session.docs_scroll(), 3);
}

#[test]
fn docs_scroll_falls_through_without_a_visible_panel() {
    let mut value = item("a", "value");
    value.documentation = Some(Documentation {
        text: "one\ntwo".to_string(),
        kind: MarkupKind::PlainText,
    });
    let mut session = CompletionSession::new(event(vec![value]), "");
    let outcome = session.handle_key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        None,
    );
    assert!(matches!(outcome, CompletionOutcome::Fallthrough));
}

#[test]
fn half_page_keys_are_not_consumed_by_docs() {
    let mut value = item("a", "value");
    value.documentation = Some(Documentation {
        text: "one\ntwo\nthree".to_string(),
        kind: MarkupKind::PlainText,
    });
    let mut session = CompletionSession::new(event(vec![value]), "");
    let outcome = session.handle_key(
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        Some((10, 5)),
    );
    assert!(matches!(outcome, CompletionOutcome::Fallthrough));
    assert_eq!(session.docs_scroll(), 0);
}

#[test]
fn docs_scroll_only_applies_with_documentation() {
    let mut item = item("a", "value");
    item.documentation = Some(Documentation {
        text: "one\ntwo\nthree".to_string(),
        kind: MarkupKind::PlainText,
    });
    let mut session = CompletionSession::new(event(vec![item]), "");
    session.handle_key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        Some((usize::MAX, 2)),
    );
    assert_eq!(session.docs_scroll(), 2);
}

mod display;
mod documentation;
mod navigation;
