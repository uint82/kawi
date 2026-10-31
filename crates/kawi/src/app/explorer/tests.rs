use std::fs;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::Mode;

use super::super::App;
use super::*;
use crate::keymap::config::KeymapLoad;
use crate::keymap::{default_leaders, Binding, BindingSource, Flavor, KeyContext, KeySequence};
use crate::test_support::temp_dir;

fn app_with_explorer(dir: &std::path::Path) -> App {
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.set_screen_size(80, 24);
    app.explorer.set_enabled(true);
    app.explorer.open(dir.to_path_buf());
    app
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("key");
}

fn leader_toggle(app: &mut App) {
    press(app, KeyCode::Char(' '));
    press(app, KeyCode::Char('e'));
    press(app, KeyCode::Char('e'));
}

fn assert_leader_toggle_round_trip(app: &mut App) {
    leader_toggle(app);
    assert!(app.explorer.is_visible());
    assert!(app.explorer.is_focused());
    leader_toggle(app);
    assert!(!app.explorer.is_visible());
    assert!(!app.explorer.is_focused());
}

fn app_with_hidden_explorer() -> App {
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.set_screen_size(80, 24);
    app.explorer.set_enabled(true);
    app
}

fn bind_leader_prefixed_command(app: &mut App) {
    let command = app.commands.find("mode.visual").expect("command");
    let binding = Binding::new(
        command,
        KeyContext::Normal,
        KeySequence::parse("<leader>pp", &default_leaders()).expect("keys"),
        BindingSource::User,
    );
    app.apply_keymaps(KeymapLoad {
        leader: default_leaders(),
        timeout: Duration::from_secs(1),
        bindings: vec![binding],
        warnings: Vec::new(),
    });
}

fn row_names(app: &App) -> Vec<&str> {
    app.explorer
        .rows()
        .iter()
        .map(|row| row.name.as_str())
        .collect()
}

#[test]
fn explorer_state_starts_hidden() {
    let state = ExplorerState::default();
    assert!(!state.is_visible());
    assert!(!state.is_focused());
    assert!(state.rows().is_empty());
}

#[test]
fn enter_expands_directories_and_escape_returns_to_the_editor() {
    let dir = temp_dir("explorer_keys");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "").unwrap();
    let mut app = app_with_explorer(&dir);
    assert_eq!(row_names(&app), ["src"]);
    press(&mut app, KeyCode::Enter);
    assert_eq!(row_names(&app), ["src", "main.rs"]);
    press(&mut app, KeyCode::Esc);
    assert!(!app.explorer.is_focused());
    assert!(app.explorer.is_visible());
    assert_eq!(app.editor.mode(), Mode::Normal);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn q_closes_the_explorer() {
    let dir = temp_dir("explorer_close");
    fs::write(dir.join("a.txt"), "").unwrap();
    let mut app = app_with_explorer(&dir);
    press(&mut app, KeyCode::Char('q'));
    assert!(!app.explorer.is_visible());
    assert!(!app.explorer.is_focused());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn leader_toggle_closes_the_focused_explorer() {
    let dir = temp_dir("explorer_leader_close");
    fs::write(dir.join("a.txt"), "").unwrap();
    let mut app = app_with_explorer(&dir);
    leader_toggle(&mut app);
    assert!(!app.explorer.is_visible());
    assert!(!app.explorer.is_focused());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn leader_toggle_opens_and_closes_without_waiting() {
    let mut app = app_with_hidden_explorer();
    assert_leader_toggle_round_trip(&mut app);
}

#[test]
fn leader_toggle_closes_with_leader_prefixed_mappings() {
    let mut app = app_with_hidden_explorer();
    bind_leader_prefixed_command(&mut app);
    assert_leader_toggle_round_trip(&mut app);
}

#[test]
fn directory_keys_move_and_collapse_the_selection() {
    let dir = temp_dir("explorer_motion");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "").unwrap();
    fs::write(dir.join("README.md"), "").unwrap();
    let mut app = app_with_explorer(&dir);
    press(&mut app, KeyCode::Char('G'));
    assert_eq!(
        app.explorer.selected_row().map(|row| row.name.as_str()),
        Some("README.md")
    );
    press(&mut app, KeyCode::Char('g'));
    assert_eq!(
        app.explorer.selected_row().map(|row| row.name.as_str()),
        Some("src")
    );
    press(&mut app, KeyCode::Char('l'));
    assert_eq!(row_names(&app), ["src", "main.rs", "README.md"]);
    press(&mut app, KeyCode::Char('h'));
    assert_eq!(row_names(&app), ["src", "README.md"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn refresh_reveals_files_created_on_disk() {
    let dir = temp_dir("explorer_refresh");
    fs::write(dir.join("alpha.txt"), "").unwrap();
    let mut app = app_with_explorer(&dir);
    fs::write(dir.join("beta.txt"), "").unwrap();
    press(&mut app, KeyCode::Char('R'));
    assert_eq!(row_names(&app), ["alpha.txt", "beta.txt"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn toggling_hidden_files_updates_the_tree() {
    let dir = temp_dir("explorer_hidden");
    fs::write(dir.join(".env"), "").unwrap();
    let mut app = app_with_explorer(&dir);
    assert!(row_names(&app).is_empty());
    press(&mut app, KeyCode::Char('.'));
    assert_eq!(row_names(&app), [".env"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn explorer_reserves_width_next_to_the_editor() {
    let dir = temp_dir("explorer_layout");
    fs::write(dir.join("a.txt"), "").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.explorer.set_enabled(true);
    app.explorer.open(dir.clone());
    let rect = app.explorer_rect().expect("explorer rect");
    assert_eq!(rect.width, 30);
    assert_eq!(rect.height, 29);
    assert_eq!(app.explorer_content_height(), 28);
    let area = app.window_area();
    assert_eq!(area.x, 30);
    assert_eq!(area.width, 70);
    app.explorer.set_width(60);
    assert_eq!(app.explorer_rect().expect("wider rect").width, 60);
    assert_eq!(app.window_area().width, 40);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn explorer_root_follows_the_current_file() {
    let dir = temp_dir("explorer_root");
    let file = dir.join("main.rs");
    fs::write(&file, "").unwrap();
    let app = App::new(
        Buffer::from_file(file.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert_eq!(
        app.explorer_root(),
        dir.canonicalize().expect("canonical root")
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn opening_the_explorer_reveals_the_current_file() {
    let dir = temp_dir("explorer_reveal");
    fs::create_dir_all(dir.join("src")).unwrap();
    let file = dir.join("src/main.rs");
    fs::write(&file, "").unwrap();
    let mut app = App::new(
        Buffer::from_file(file.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    app.set_screen_size(80, 24);
    app.explorer.set_enabled(true);
    app.open_explorer_at(dir.clone());
    assert!(app.explorer.is_visible());
    assert_eq!(
        app.explorer.selected_row().map(|row| row.path.as_path()),
        Some(file.as_path())
    );
    assert_eq!(row_names(&app), ["src", "main.rs"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn explorer_is_disabled_by_default() {
    let dir = temp_dir("explorer_opt_in");
    fs::write(dir.join("a.txt"), "").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.set_screen_size(80, 24);
    assert!(!app.explorer.is_enabled());
    app.open_explorer_at(dir.clone());
    assert!(!app.explorer.is_visible());
    assert_eq!(app.last_message(), Some("Project explorer is disabled"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn enabling_the_explorer_allows_it_to_open() {
    let dir = temp_dir("explorer_enabled");
    fs::write(dir.join("a.txt"), "").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.set_screen_size(80, 24);
    let spec = crate::options::lookup("explorer_enabled").expect("option");
    app.apply_option(spec, crate::options::OptionValue::Bool(true))
        .expect("enable explorer");
    app.open_explorer_at(dir.clone());
    assert!(app.explorer.is_visible());
    assert!(app.explorer.is_focused());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn explorer_closes_when_the_screen_is_too_narrow() {
    let dir = temp_dir("explorer_narrow");
    fs::write(dir.join("a.txt"), "").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.set_screen_size(80, 24);
    app.explorer.set_enabled(true);
    app.explorer.open(dir.clone());
    assert!(app.explorer.is_visible());
    app.set_screen_size(39, 24);
    assert!(!app.explorer.is_visible());
    fs::remove_dir_all(dir).unwrap();
}
