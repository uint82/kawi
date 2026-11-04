use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn files() -> Picker {
    let mut picker = Picker::files();
    picker.set_files(vec![
        "src/main.rs".to_string(),
        "src/lib.rs".to_string(),
        "README.md".to_string(),
    ]);
    picker
}

#[test]
fn files_filter_and_rank_fuzzy_matches() {
    let mut picker = files();
    for character in "main".chars() {
        picker.handle_key(key(KeyCode::Char(character)));
    }
    assert_eq!(picker.rows().len(), 1);
    assert_eq!(picker.rows()[0].path, "src/main.rs");
}

#[test]
fn cached_candidates_preserve_smart_case_and_empty_query() {
    let mut picker = Picker::files();
    picker.set_files(vec![
        "README.md".to_string(),
        "readme.txt".to_string(),
        "src/main.rs".to_string(),
    ]);
    picker.set_query("README");
    assert_eq!(picker.rows().len(), 1);
    assert_eq!(picker.rows()[0].path, "README.md");
    picker.set_query("readme");
    assert_eq!(picker.rows().len(), 2);
    picker.set_query("");
    assert_eq!(picker.rows().len(), 3);
    assert_eq!(picker.rows()[0].path, "README.md");
}

#[test]
fn locations_filter_and_open() {
    let rows = vec![
        PickerRow {
            display: "src/main.rs:1:1".to_string(),
            path: "src/main.rs".to_string(),
            line: Some(1),
            column: Some(1),
            buffer: None,
            secondary: None,
            deprecated: false,
            id: None,
        },
        PickerRow {
            display: "src/lib.rs:4:2".to_string(),
            path: "src/lib.rs".to_string(),
            line: Some(4),
            column: Some(2),
            buffer: None,
            secondary: None,
            deprecated: false,
            id: None,
        },
    ];
    let mut picker = Picker::locations("References", rows);
    assert_eq!(picker.kind(), PickerKind::Locations);
    assert_eq!(picker.rows().len(), 2);
    assert_eq!(picker.rows()[0].path, "src/main.rs");
    for character in "lib".chars() {
        picker.handle_key(key(KeyCode::Char(character)));
    }
    assert_eq!(picker.rows().len(), 1);
    let outcome = picker.handle_key(key(KeyCode::Enter));
    assert!(matches!(outcome, PickerOutcome::Open(row) if row.path == "src/lib.rs"));
}

#[test]
fn replacing_locations_preserves_the_selected_row() {
    let rows = vec![
        PickerRow {
            display: "a.rs:1:1 first".to_string(),
            path: "a.rs".to_string(),
            line: Some(1),
            column: Some(1),
            buffer: None,
            secondary: None,
            deprecated: false,
            id: None,
        },
        PickerRow {
            display: "b.rs:2:1 second".to_string(),
            path: "b.rs".to_string(),
            line: Some(2),
            column: Some(1),
            buffer: None,
            secondary: None,
            deprecated: false,
            id: None,
        },
    ];
    let mut picker = Picker::locations("Problems", rows.clone());
    picker.handle_key(key(KeyCode::Down));
    let mut updated = vec![PickerRow {
        display: "a.rs:3:1 new".to_string(),
        path: "a.rs".to_string(),
        line: Some(3),
        column: Some(1),
        buffer: None,
        secondary: None,
        deprecated: false,
        id: None,
    }];
    updated.extend(rows);
    updated[2].display = "b.rs:2:1 updated message".to_string();
    picker.set_locations(updated);
    assert_eq!(picker.selected(), Some(2));
    assert_eq!(picker.rows()[2].display, "b.rs:2:1 updated message");
}

#[test]
fn enter_opens_the_selected_row() {
    let mut picker = files();
    picker.handle_key(key(KeyCode::Down));
    let outcome = picker.handle_key(key(KeyCode::Enter));
    assert!(matches!(outcome, PickerOutcome::Open(row) if row.path == "src/lib.rs"));
}

#[test]
fn query_editing_supports_cursor_and_word_deletion() {
    let mut picker = files();
    for character in "main".chars() {
        picker.handle_key(key(KeyCode::Char(character)));
    }
    picker.handle_key(key(KeyCode::Backspace));
    assert_eq!(picker.query(), "mai");
    picker.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(picker.query(), "");
    for character in "src lib".chars() {
        picker.handle_key(key(KeyCode::Char(character)));
    }
    picker.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    assert_eq!(picker.query(), "src ");
}

#[test]
fn grep_returns_query_changes_and_builds_rows() {
    let mut picker = Picker::grep();
    let outcome = picker.handle_key(key(KeyCode::Char('n')));
    assert_eq!(outcome, PickerOutcome::QueryChanged);
    assert!(picker.loading());
    picker.set_hits(vec![GrepHit {
        path: "src/main.rs".to_string(),
        line: 3,
        column: 5,
        text: "needle".to_string(),
    }]);
    assert_eq!(picker.rows().len(), 1);
    assert_eq!(picker.rows()[0].display, "src/main.rs:3:5: needle");
}

#[test]
fn buffers_filter_by_name() {
    let mut picker = Picker::buffers(vec![
        (1, "src/main.rs".to_string()),
        (2, "README.md".to_string()),
    ]);
    for character in "read".chars() {
        picker.handle_key(key(KeyCode::Char(character)));
    }
    assert_eq!(picker.rows().len(), 1);
    assert_eq!(picker.rows()[0].buffer, Some(2));
}

#[test]
fn navigation_wraps_at_the_edges() {
    let mut picker = files();
    picker.handle_key(key(KeyCode::Down));
    assert_eq!(picker.selected(), Some(1));
    picker.handle_key(key(KeyCode::Down));
    assert_eq!(picker.selected(), Some(2));
    picker.handle_key(key(KeyCode::Down));
    assert_eq!(picker.selected(), Some(0));
    picker.handle_key(key(KeyCode::Up));
    assert_eq!(picker.selected(), Some(2));
    picker.handle_key(key(KeyCode::Up));
    assert_eq!(picker.selected(), Some(1));
}

#[test]
fn escape_cancels() {
    let mut picker = files();
    assert_eq!(picker.handle_key(key(KeyCode::Esc)), PickerOutcome::Cancel);
}

#[test]
fn tab_moves_forward_and_backtab_moves_backward() {
    let mut picker = files();
    assert_eq!(picker.selected(), Some(0));
    picker.handle_key(key(KeyCode::Tab));
    assert_eq!(picker.selected(), Some(1));
    picker.handle_key(key(KeyCode::Tab));
    assert_eq!(picker.selected(), Some(2));
    picker.handle_key(key(KeyCode::BackTab));
    assert_eq!(picker.selected(), Some(1));
    picker.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT));
    assert_eq!(picker.selected(), Some(0));
    picker.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT));
    assert_eq!(picker.selected(), Some(2));
    picker.handle_key(key(KeyCode::Tab));
    assert_eq!(picker.selected(), Some(0));
}
