use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::Mode;

use crate::keymap::Flavor;

mod layout;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app() -> App {
    let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
    app.set_screen_size(80, 24);
    app.keyguide_delay = std::time::Duration::ZERO;
    app
}

fn items(app: &App) -> Vec<String> {
    let data = app.overlay_data().expect("overlay");
    let menu = data.key_menu.as_ref().expect("key menu");
    menu.rows
        .iter()
        .map(|row| format!("{}  {}", row.key_label(), row.description))
        .collect()
}

fn root_items() -> [&'static str; 8] {
    [
        "c  code tools",
        "d  diagnostics",
        "e  explorer",
        "f  find",
        "r  rename",
        "s  symbols",
        "t  tasks and terminal",
        "v  git actions",
    ]
}

fn find_items() -> [&'static str; 5] {
    [
        "f/p  project files",
        "k  open buffers",
        "r  search and replace",
        "s  live grep",
        "w  grep current word",
    ]
}

fn bind_leader_mapping(app: &mut App, keys: &str) {
    let command = format!("nmap {keys} mode.visual");
    app.run_command(kawi_editor::parse(&command))
        .expect("mapping");
}

#[test]
fn root_menu_lists_every_group() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(items(&app), root_items());
}

#[test]
fn resolver_pending_leader_shows_the_root_menu() {
    let mut app = app();
    bind_leader_mapping(&mut app, "<leader>pp");
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(items(&app), root_items());
}

#[test]
fn resolver_pending_group_shows_the_group_menu() {
    let mut app = app();
    bind_leader_mapping(&mut app, "<leader>ffx");
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    app.handle_key(key(KeyCode::Char('f'))).expect("group");
    assert_eq!(items(&app), find_items());
}

#[test]
fn group_menu_replaces_the_root_menu() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    app.handle_key(key(KeyCode::Char('f'))).expect("group");
    assert_eq!(items(&app), find_items());
}

#[test]
fn breadcrumb_tracks_the_active_group() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(
        app.overlay_data()
            .expect("overlay")
            .key_menu
            .as_ref()
            .expect("key menu")
            .breadcrumb,
        ""
    );
    app.handle_key(key(KeyCode::Char('f'))).expect("group");
    assert_eq!(
        app.overlay_data()
            .expect("overlay")
            .key_menu
            .as_ref()
            .expect("key menu")
            .breadcrumb,
        "f"
    );
}

#[test]
fn visual_menu_only_lists_supported_groups() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('v'))).expect("visual");
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert_eq!(items(&app), ["c  code tools", "f  find"]);
}

#[test]
fn backslash_leader_uses_the_same_menu() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('\\'))).expect("leader");
    assert_eq!(items(&app), root_items());
    app.handle_key(key(KeyCode::Char('f'))).expect("group");
    assert_eq!(
        items(&app).first().map(String::as_str),
        Some("f/p  project files")
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
    assert_eq!(items(&app), ["c  code tools", "f  find"]);
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
    app.handle_key(key(KeyCode::Char('Q'))).expect("unknown");
    assert!(app.overlay_data().is_none());
}

#[test]
fn operator_prefix_lists_followups() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('c'))).expect("operator");
    assert_eq!(
        app.overlay_data()
            .expect("overlay")
            .key_menu
            .as_ref()
            .expect("key menu")
            .breadcrumb,
        "c"
    );
    let items = items(&app);
    assert!(items.contains(&"w  change word".to_string()));
    assert!(items.contains(&"i  inner text object".to_string()));
    assert!(items.contains(&"a  around text object".to_string()));
}

#[test]
fn large_operator_menu_uses_two_columns() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('d'))).expect("operator");
    let data = app.overlay_data().expect("overlay");
    let menu = data.key_menu.as_ref().expect("key menu");
    assert_eq!(menu.columns.len(), 2);
    assert!(menu.rect.height < 24);
}

#[test]
fn second_key_updates_the_guide() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('c'))).expect("operator");
    app.handle_key(key(KeyCode::Char('i'))).expect("scope");
    assert_eq!(
        app.overlay_data()
            .expect("overlay")
            .key_menu
            .as_ref()
            .expect("key menu")
            .breadcrumb,
        "c i"
    );
    let items = items(&app);
    assert!(items.contains(&"w  inner word".to_string()));
    assert!(items.contains(&"(/)  inner parentheses".to_string()));
}

#[test]
fn completing_a_sequence_dismisses_the_guide() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('d'))).expect("operator");
    assert!(app.overlay_data().is_some());
    app.handle_key(key(KeyCode::Char('d'))).expect("line");
    assert!(app.overlay_data().is_none());
}

#[test]
fn counts_do_not_change_the_guide() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('2'))).expect("count");
    assert!(app.overlay_data().is_none());
    app.handle_key(key(KeyCode::Char('d'))).expect("operator");
    assert_eq!(
        app.overlay_data()
            .expect("overlay")
            .key_menu
            .as_ref()
            .expect("key menu")
            .breadcrumb,
        "d"
    );
}

#[test]
fn dynamic_prefix_shows_a_hint() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('f'))).expect("find");
    let data = app.overlay_data().expect("overlay");
    let menu = data.key_menu.as_ref().expect("key menu");
    assert_eq!(menu.breadcrumb, "f");
    assert_eq!(menu.hint, Some("target character"));
    assert!(menu.rows.is_empty());
}

#[test]
fn visual_object_prefix_lists_objects() {
    let mut app = app();
    app.handle_key(key(KeyCode::Char('v'))).expect("visual");
    app.handle_key(key(KeyCode::Char('i'))).expect("object");
    let items = items(&app);
    assert!(items.contains(&"w  inner word".to_string()));
}

#[test]
fn keyguide_can_be_disabled() {
    let mut app = app();
    app.keyguide_enabled = false;
    app.handle_key(key(KeyCode::Char('c'))).expect("operator");
    assert!(app.overlay_data().is_none());
    app.handle_key(key(KeyCode::Char(' '))).expect("leader");
    assert!(app.overlay_data().is_none());
}

#[test]
fn keyguide_waits_for_the_delay() {
    let mut app = app();
    app.keyguide_delay = std::time::Duration::from_millis(500);
    app.handle_key(key(KeyCode::Char('c'))).expect("operator");
    assert!(app.overlay_data().is_none());
    app.keyguide_since = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
    app.sync_keyguide();
    assert!(app.overlay_data().is_some());
}
