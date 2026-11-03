use super::*;
use crate::keymap::Flavor;
use kawi_buffer::Buffer;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_lsp_status_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn executable(path: &Path) {
    fs::write(path, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn config(missing: &Path, available: &Path) -> String {
    format!(
        "[server.rust-analyzer]\nenabled = false\n\
         [server.pyright]\nenabled = false\n\
         [server.ruff]\nenabled = false\n\
         [server.fake]\ncommand = {missing:?}\nargs = [\"--stdio\"]\n\
         [server.idle]\ncommand = {available:?}\n\
         [server.unused]\ncommand = \"kawi-unused-server\"\n\n\
         [[language]]\nname = \"rust\"\nservers = [\"fake\", \"idle\"]\n\n\
         [[language]]\nname = \"python\"\nservers = [\"fake\"]\n"
    )
}

fn loaded(name: &str) -> (PathBuf, PathBuf, Languages) {
    let dir = temp_dir(name);
    let missing = dir.join("missing-server");
    let available = dir.join("available-server");
    executable(&available);
    let path = dir.join("languages.toml");
    fs::write(&path, config(&missing, &available)).unwrap();
    let languages = Languages::load(&path);
    (dir, missing, languages)
}

fn row<'a, 'b>(rows: &'a [StatusRow<'b>], server: &str) -> &'a StatusRow<'b> {
    rows.iter()
        .find(|row| row.server == server)
        .expect("status row")
}

#[test]
fn rows_report_configuration_states_and_languages() {
    let (dir, missing, languages) = loaded("rows");
    let rows = build_rows(&languages, &BTreeMap::new(), &BTreeSet::new());
    assert_eq!(row(&rows, "fake").state, "missing");
    assert_eq!(row(&rows, "fake").languages, "rust, python");
    assert_eq!(
        row(&rows, "fake").command,
        format!("{} --stdio", missing.display())
    );
    assert!(row(&rows, "fake").path.is_none());
    assert_eq!(row(&rows, "idle").state, "idle");
    assert!(row(&rows, "idle").path.is_some());
    assert_eq!(row(&rows, "unused").state, "unused");
    assert_eq!(row(&rows, "rust-analyzer").state, "disabled");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn runtime_state_overrides_configuration_state() {
    let (dir, _, languages) = loaded("runtime");
    let mut states = BTreeMap::new();
    states.insert("fake".to_string(), ServerState::Ready);
    let rows = build_rows(&languages, &states, &BTreeSet::new());
    assert_eq!(row(&rows, "fake").state, "ready");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn report_renders_table_paths_and_details() {
    let (dir, _, languages) = loaded("report");
    let config_path = dir.join("languages.toml");
    let states = BTreeMap::new();
    let mut messages = BTreeMap::new();
    messages.insert("fake".to_string(), "boom\nline".to_string());
    let lines = report(&StatusContext {
        languages: &languages,
        states: &states,
        messages: &messages,
        warnings_path: Some(&config_path),
        current: None,
        progress: &[],
    });
    assert_eq!(lines[0], TITLE);
    assert_eq!(lines[1], "");
    assert!(lines[2].starts_with("SERVER"));
    assert!(lines.iter().any(|line| line.starts_with("fake")));
    assert!(lines.iter().any(|line| line == DETAILS));
    assert!(lines.iter().any(|line| line == "fake: boom line"));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("idle: ") && line.contains("available-server")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn report_marks_current_buffer_servers() {
    let (dir, _, languages) = loaded("current");
    let current = current_servers(&languages, Some(Path::new("src/main.rs")));
    let rows = build_rows(&languages, &BTreeMap::new(), &current);
    assert!(row(&rows, "fake").current);
    assert!(row(&rows, "idle").current);
    assert!(!row(&rows, "unused").current);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn report_renders_active_progress() {
    let (dir, _, languages) = loaded("progress");
    let states = BTreeMap::new();
    let messages = BTreeMap::new();
    let progress = vec!["fake: Indexing 50%".to_string()];
    let lines = report(&StatusContext {
        languages: &languages,
        states: &states,
        messages: &messages,
        warnings_path: None,
        current: None,
        progress: &progress,
    });
    assert!(lines.iter().any(|line| line == PROGRESS));
    assert!(lines.iter().any(|line| line == "fake: Indexing 50%"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn report_renders_configuration_warnings() {
    let dir = temp_dir("warnings");
    let path = dir.join("languages.toml");
    fs::write(
        &path,
        "[server.fake]\ncommand = \"\"\n\n[[language]]\nname = \"rust\"\nservers = [\"fake\"]\n",
    )
    .unwrap();
    let languages = Languages::load(&path);
    assert!(!languages.warnings().is_empty());
    let states = BTreeMap::new();
    let messages = BTreeMap::new();
    let lines = report(&StatusContext {
        languages: &languages,
        states: &states,
        messages: &messages,
        warnings_path: Some(&path),
        current: None,
        progress: &[],
    });
    assert!(lines.iter().any(|line| line == WARNINGS));
    assert!(lines
        .iter()
        .any(|line| line.contains("has an empty command")));
    assert!(lines
        .iter()
        .any(|line| line.contains(path.to_str().unwrap())));
    let _ = fs::remove_dir_all(dir);
}

fn status_app(name: &str) -> (PathBuf, App) {
    let dir = temp_dir(name);
    let path = dir.join("languages.toml");
    let missing = dir.join("missing-server");
    let available = dir.join("available-server");
    executable(&available);
    fs::write(&path, config(&missing, &available)).unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.lsp.load(&path, app.events.wake());
    (dir, app)
}

#[test]
fn status_report_includes_active_progress() {
    let (dir, mut app) = status_app("progress");
    app.lsp.set_progress_for_test();
    let lines = app.lsp.status_report();
    assert!(lines.iter().any(|line| line == PROGRESS));
    assert!(lines.iter().any(|line| line.contains("Indexing")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn command_shows_status_in_internal_buffer() {
    let (dir, mut app) = status_app("command");
    app.execute_command_text("lsp status").unwrap();
    assert!(app.editor.current_buffer_is_internal());
    let number = app.editor.buffer_number();
    let text = app.editor.buffer_text(number).expect("status buffer");
    assert!(text.starts_with(TITLE));
    let windows = app.windows.len();
    app.execute_command_text("lsp status").unwrap();
    assert_eq!(app.editor.buffer_number(), number);
    assert_eq!(app.windows.len(), windows);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtin_command_shows_status_in_internal_buffer() {
    let (dir, mut app) = status_app("builtin");
    let id = app.commands.find("lsp.status").expect("command");
    app.dispatch_command_id(id, None).unwrap();
    assert!(app.editor.current_buffer_is_internal());
    let text = app
        .editor
        .buffer_text(app.editor.buffer_number())
        .expect("status buffer");
    assert!(text.starts_with(TITLE));
    let _ = fs::remove_dir_all(dir);
}
