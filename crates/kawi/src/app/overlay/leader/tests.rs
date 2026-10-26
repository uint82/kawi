use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::Mode;

use crate::keymap::Flavor;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app() -> App {
    let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
    app.set_screen_size(80, 24);
    app
}

fn items(app: &App) -> Vec<String> {
    let data = app.overlay_data().expect("overlay");
    let leader = data.leader.as_ref().expect("leader");
    leader
        .rows
        .iter()
        .map(|row| format!("{}  {}", row.key, row.description))
        .collect()
}

fn root_items() -> [&'static str; 5] {
    ["c  code", "f  find", "r  rename", "t  tasks", "v  git"]
}

#[test]
fn root_menu_lists_every_group() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(items(&app), root_items());
}

#[test]
fn group_menu_replaces_the_root_menu() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    app.handle_key(key(KeyCode::Char('f'))).expect("group");
    assert_eq!(
        items(&app),
        [
            "f  find project file",
            "k  find buffer",
            "r  find and replace",
            "s  find in project",
            "w  find current word",
        ]
    );
}

#[test]
fn breadcrumb_tracks_the_active_group() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(
        app.overlay_data()
            .expect("overlay")
            .leader
            .as_ref()
            .expect("leader")
            .group,
        None
    );
    app.handle_key(key(KeyCode::Char('f'))).expect("group");
    assert_eq!(
        app.overlay_data()
            .expect("overlay")
            .leader
            .as_ref()
            .expect("leader")
            .group,
        Some('f')
    );
}

#[test]
fn visual_menu_only_lists_supported_groups() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('v'))).expect("visual");
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(items(&app), ["c  code", "f  find"]);
}

#[test]
fn backslash_leader_uses_the_same_menu() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('\\'))).expect("leader");
    assert_eq!(items(&app), root_items());
    app.handle_key(key(KeyCode::Char('f'))).expect("group");
    assert_eq!(
        items(&app).first().map(String::as_str),
        Some("f  find project file")
    );
}

#[test]
fn completing_a_command_dismisses_the_menu() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    app.handle_key(key(KeyCode::Char('t'))).expect("group");
    app.handle_key(key(KeyCode::Char('c'))).expect("command");
    assert!(app.overlay_data().is_none());
}

#[test]
fn pressing_space_again_dismisses_the_root_menu() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(items(&app), root_items());
    app.handle_key(key(KeyCode::Char(' '))).expect("toggle");
    assert!(app.overlay_data().is_none());
    assert_eq!(app.editor_mode(), Mode::Normal);
}

#[test]
fn pressing_space_again_keeps_visual_mode() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('v'))).expect("visual");
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(items(&app), ["c  code", "f  find"]);
    app.handle_key(key(KeyCode::Char(' '))).expect("toggle");
    assert!(app.overlay_data().is_none());
    assert_eq!(app.editor_mode(), Mode::Visual);
}

#[test]
fn canceling_and_unknown_keys_dismiss_the_menu() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    app.handle_key(key(KeyCode::Esc)).expect("cancel");
    assert!(app.overlay_data().is_none());

    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    app.handle_key(key(KeyCode::Char('z'))).expect("unknown");
    assert!(app.overlay_data().is_none());
}

#[test]
fn short_menus_use_one_column() {
    let rows: Vec<LeaderRow> = ('a'..='j')
        .map(|key| LeaderRow {
            key,
            description: "item",
        })
        .collect();
    let columns = leader_columns(&rows, 58);
    assert_eq!(columns.len(), 1);
    assert_eq!(columns[0].len, 10);
}

#[test]
fn growing_menus_use_two_columns() {
    let rows: Vec<LeaderRow> = ('a'..='k')
        .map(|key| LeaderRow {
            key,
            description: "item",
        })
        .collect();
    let columns = leader_columns(&rows, 58);
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].start, 0);
    assert_eq!(columns[0].len, 6);
    assert_eq!(columns[1].start, 6);
    assert_eq!(columns[1].len, 5);
}

#[test]
fn wide_two_column_menus_fall_back_to_one_column() {
    let rows: Vec<LeaderRow> = ('a'..='k')
        .map(|key| LeaderRow {
            key,
            description: "abcdefghijklmnopqrstuvwxyz1234",
        })
        .collect();
    let columns = leader_columns(&rows, 58);
    assert_eq!(columns.len(), 1);
}

#[test]
fn menu_is_right_aligned_above_the_bottom_edge() {
    let rows = [
        LeaderRow {
            key: 'f',
            description: "find",
        },
        LeaderRow {
            key: 'v',
            description: "git",
        },
    ];
    let columns = leader_columns(&rows, 58);
    let rect = leader_rect(Rect::new(0, 0, 80, 24), None, &columns);
    assert_eq!(rect.width, 9);
    assert_eq!(rect.height, 5);
    assert_eq!(rect.x, 70);
    assert_eq!(rect.y, 18);
}
