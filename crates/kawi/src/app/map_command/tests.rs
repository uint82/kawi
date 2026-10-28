use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::{Action, Mode};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::super::App;
use crate::command::CommandId;
use crate::keymap::config::KeymapLoad;
use crate::keymap::{
    default_leaders, Binding, BindingSource, Flavor, KeyChord, KeyContext, KeySequence, Keymap,
};

fn app() -> App {
    App::new(Buffer::from_string("alpha\nbeta\n"), Flavor::Vim)
}

fn key(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
}

fn command(app: &App, name: &str) -> CommandId {
    app.commands.find(name).expect("command")
}

fn binding(app: &App, name: &str, mode: KeyContext, keys: &str) -> Binding {
    Binding::new(
        command(app, name),
        mode,
        KeySequence::parse(keys, &default_leaders()).expect("keys"),
        BindingSource::User,
    )
}

fn load(bindings: Vec<Binding>) -> KeymapLoad {
    KeymapLoad {
        leader: default_leaders(),
        timeout: Duration::ZERO,
        bindings,
        warnings: Vec::new(),
    }
}

fn execute(app: &mut App, text: &str) {
    app.run_command(kawi_editor::parse(text)).unwrap();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kawi_map_command_{}_{}_{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test"),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn configured_mapping_dispatches_command() {
    let mut app = app();
    let load = load(vec![binding(
        &app,
        "edit.delete_line",
        KeyContext::Normal,
        "gx",
    )]);
    app.apply_keymaps(load);
    app.handle_key(key('g')).unwrap();
    assert_eq!(app.keymaps.pending_text().as_deref(), Some("g"));
    app.handle_key(key('x')).unwrap();
    assert_eq!(app.editor.text_lines(), ["beta"]);
    assert_eq!(app.keymaps.pending_text(), None);
}

#[test]
fn insert_mapping_prevents_text() {
    let mut app = app();
    let load = load(vec![binding(&app, "mode.normal", KeyContext::Insert, "jk")]);
    app.apply_keymaps(load);
    app.editor.apply(Action::EnterInsert);
    app.handle_key(key('j')).unwrap();
    app.handle_key(key('k')).unwrap();
    assert_eq!(app.editor.mode(), Mode::Normal);
    assert_eq!(app.editor.text_lines(), ["alpha", "beta"]);
}

#[test]
fn send_keys_replays_through_resolver() {
    let mut app = app();
    let send = command(&app, "keys.send");
    let send = Binding::new(
        send,
        KeyContext::Normal,
        KeySequence::parse("Q", &default_leaders()).expect("keys"),
        BindingSource::User,
    )
    .with_args(Some(toml::Value::String("gx".to_string())));
    let target = binding(&app, "mode.visual", KeyContext::Normal, "gx");
    app.apply_keymaps(load(vec![send, target]));
    app.handle_key(key('Q')).unwrap();
    assert_eq!(app.editor.mode(), Mode::Visual);
}

#[test]
fn ambiguous_mapping_times_out_to_shorter_sequence() {
    let mut app = app();
    let load = load(vec![
        binding(&app, "mode.visual", KeyContext::Normal, "g"),
        binding(&app, "edit.delete_line", KeyContext::Normal, "gg"),
    ]);
    app.apply_keymaps(load);
    app.handle_key(key('g')).unwrap();
    assert_eq!(app.editor.mode(), Mode::Normal);
    assert_eq!(app.flush_mapping_timeout().unwrap(), Some(false));
    assert_eq!(app.editor.mode(), Mode::Visual);
}

#[test]
fn runtime_nmap_and_nunmap() {
    let mut app = app();
    execute(&mut app, "nmap gx edit.delete_line");
    app.handle_key(key('g')).unwrap();
    app.handle_key(key('x')).unwrap();
    assert_eq!(app.editor.text_lines(), ["beta"]);

    execute(&mut app, "nunmap gx");
    app.handle_key(key('g')).unwrap();
    app.handle_key(key('x')).unwrap();
    assert_eq!(app.editor.text_lines(), ["beta"]);
}

#[test]
fn runtime_map_lists_bindings() {
    let mut app = app();
    execute(&mut app, "nmap gx edit.delete_line");
    execute(&mut app, "map");
    let lines = app.pager_lines().expect("pager");
    assert!(lines.iter().any(|line| line.contains("gx")));
    assert!(lines.iter().any(|line| line.contains("edit.delete_line")));
}

#[test]
fn map_reload_reads_the_file() {
    let dir = temp_dir("reload");
    let path = dir.join("keymaps.toml");
    fs::write(&path, "[normal]\n\"gx\" = \"edit.delete_line\"\n").unwrap();
    let mut app = app();
    app.keymaps_path = Some(path);
    execute(&mut app, "map reload");
    app.handle_key(key('g')).unwrap();
    app.handle_key(key('x')).unwrap();
    assert_eq!(app.editor.text_lines(), ["beta"]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn commands_lists_command_names() {
    let mut app = app();
    execute(&mut app, "commands");
    let lines = app.pager_lines().expect("pager");
    assert!(lines.iter().any(|line| line.contains("file.save")));
    assert!(lines.iter().any(|line| line.contains("keys.send")));
}

#[test]
fn flavor_change_preserves_configured_leader() {
    let mut app = app();
    let comma = KeyChord::from_event(key(','));
    app.apply_keymaps(KeymapLoad {
        leader: vec![comma],
        timeout: Duration::ZERO,
        bindings: Vec::new(),
        warnings: Vec::new(),
    });
    app.apply_flavor(Flavor::Vim).unwrap();
    assert!(app.keymap.is_leader_prefix(key(','), Mode::Normal));
}

#[test]
fn unknown_mapping_command_reports_error() {
    let mut app = app();
    execute(&mut app, "nmap gx missing.command");
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("Unknown command")));
}

#[test]
fn runtime_leader_mapping_covers_every_leader() {
    let mut app = app();
    execute(&mut app, "nmap <leader>gz picker.files");
    assert_eq!(app.keymaps.bindings().len(), 2);
    execute(&mut app, "nunmap <leader>gz");
    assert_eq!(app.keymaps.bindings().len(), 2);
    assert!(app
        .keymaps
        .bindings()
        .iter()
        .all(|binding| binding.command == command(&app, "none")));
}
