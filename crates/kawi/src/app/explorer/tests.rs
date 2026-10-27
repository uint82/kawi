use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::Mode;

use super::super::App;
use super::*;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

fn app_with_explorer(dir: &std::path::Path) -> App {
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.set_screen_size(80, 24);
    app.explorer.open(dir.to_path_buf());
    app
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("key");
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
