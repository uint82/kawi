use super::*;
use kawi_syntax::SyntaxGroup;
use kawi_tui::markdown::{self, MarkdownStyle};

fn markdown_document(text: &str) -> Documentation {
    Documentation {
        text: text.to_string(),
        kind: MarkupKind::Markdown,
    }
}

#[test]
fn selected_untagged_documentation_uses_the_buffer_language() {
    let mut value = item("a", "value");
    value.documentation = Some(markdown_document("```\nlet answer = 1;\n```"));
    let session = CompletionSession::new(event(vec![value]), "");
    let document = session.selected_document().expect("prepared document");
    let lines = markdown::render_document(document, 80);
    assert!(lines[0]
        .spans
        .iter()
        .any(|span| span.style == MarkdownStyle::Syntax(SyntaxGroup::Keyword)));
}

#[test]
fn resolved_documentation_is_prepared_before_rendering() {
    let mut value = item("a", "value");
    value.resolve = true;
    let mut session = CompletionSession::new(event(vec![value]), "");
    let value = session.needs_resolve().expect("resolve").clone();
    session.start_resolve(7, &value);
    assert!(session.apply_resolved(ResolvedItem {
        request: 7,
        item_id: "a".to_string(),
        detail: None,
        documentation: Some(markdown_document("```rust\nfn value() {}\n```")),
        additional: Vec::new(),
    }));
    let document = session.selected_document().expect("prepared document");
    assert!(markdown::render_document(document, 80)
        .iter()
        .flat_map(|line| &line.spans)
        .any(|span| matches!(span.style, MarkdownStyle::Syntax(_))));
}

#[test]
fn moving_the_selection_prepares_the_new_documentation() {
    let mut second = item("b", "beta");
    second.documentation = Some(markdown_document("```rust\nfn beta() {}\n```"));
    let mut session = CompletionSession::new(event(vec![item("a", "alpha"), second]), "");
    assert!(session.selected_document().is_none());
    session.handle_key(
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        None,
    );
    assert!(session.selected_document().is_some());
}

#[test]
fn resolved_documentation_replaces_initial_documentation() {
    let mut value = item("a", "value");
    value.resolve = true;
    value.documentation = Some(markdown_document("initial"));
    let mut session = CompletionSession::new(event(vec![value]), "");
    let value = session.needs_resolve().expect("resolve").clone();
    session.start_resolve(7, &value);
    assert!(session.apply_resolved(ResolvedItem {
        request: 7,
        item_id: "a".to_string(),
        detail: None,
        documentation: Some(markdown_document("```rust\nfn resolved() {}\n```")),
        additional: Vec::new(),
    }));
    let document = session.selected_document().expect("prepared document");
    let text = markdown::render_document(document, 80)
        .iter()
        .map(|line| line.text())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(text, "fn resolved() {}");
}
