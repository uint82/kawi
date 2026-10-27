use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn press(session: &mut CompletionSession, key: KeyEvent) -> CompletionOutcome {
    let up = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
    let down = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);
    session.handle_key(key, up, down, None)
}

fn selected_id(session: &CompletionSession) -> Option<&str> {
    session.selected_item().map(|item| item.id.as_str())
}

#[test]
fn tab_moves_down_and_wraps() {
    let mut session =
        CompletionSession::new(event(vec![item("a", "alpha"), item("b", "beta")]), "");
    press(
        &mut session,
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
    );
    assert_eq!(selected_id(&session), Some("b"));
    press(
        &mut session,
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
    );
    assert_eq!(selected_id(&session), Some("a"));
}

#[test]
fn shift_tab_moves_up_and_wraps() {
    let mut session =
        CompletionSession::new(event(vec![item("a", "alpha"), item("b", "beta")]), "");
    press(
        &mut session,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
    );
    assert_eq!(selected_id(&session), Some("b"));
    press(
        &mut session,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
    );
    assert_eq!(selected_id(&session), Some("a"));
}

#[test]
fn enter_accepts_the_selection() {
    let mut session = CompletionSession::new(event(vec![item("a", "alpha")]), "");
    let outcome = press(
        &mut session,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert!(matches!(outcome, CompletionOutcome::Accept));
}
