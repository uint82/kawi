use super::*;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;
use kawi_buffer::Buffer;
use kawi_editor::Action;
use std::fs;
use std::time::{Duration, Instant};

fn app_with_formatter(name: &str, command: &str) -> (PathBuf, App) {
    let dir = temp_dir(name);
    let path = dir.join("main.rs");
    let mut buffer = Buffer::from_string("fn main() {}\n");
    buffer.path = Some(path.to_string_lossy().into_owned());
    let mut app = App::new(buffer, Flavor::Vim);
    let config = dir.join("languages.toml");
    fs::write(
        &config,
        format!(
            "[[language]]\nname = \"rust\"\nservers = []\nformatter = {{ command = {command:?}, args = [\"-c\", \"tr a-z A-Z\"] }}\n"
        ),
    )
    .unwrap();
    app.lsp.load(&config, app.events.wake());
    (dir, app)
}

#[test]
fn document_formatting_has_no_range() {
    let app = App::new(Buffer::from_string("one\ntwo\n"), Flavor::Vim);
    assert_eq!(app.formatting_range(), None);
}

#[test]
fn visual_selection_formats_whole_lines() {
    let mut app = App::new(Buffer::from_string("one\ntwo\n"), Flavor::Vim);
    app.editor.apply(Action::EnterVisualLine);
    app.editor.apply(Action::MoveDown);
    let range = app.formatting_range().expect("range");
    assert_eq!(range.0.line, 0);
    assert_eq!(range.0.column, 0);
    assert_eq!(range.1.line, 1);
    assert_eq!(range.1.column, 3);
}

fn wait_for_formatter(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && app.pending_format.is_some() {
        app.poll_formatter();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.pending_format.is_none(), "formatter timed out");
}

#[cfg(unix)]
#[test]
fn external_formatter_updates_the_buffer() {
    let (dir, mut app) = app_with_formatter("external_format", "sh");
    assert!(!app.request_format().unwrap());
    assert!(app.pending_format.is_some());
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("fn main() {}\n"));
    wait_for_formatter(&mut app);
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("FN MAIN() {}\n"));
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn external_formatter_failure_keeps_the_buffer() {
    let (dir, mut app) = app_with_formatter("external_format_failure", "sh");
    let config = dir.join("languages.toml");
    fs::write(
        &config,
        "[[language]]\nname = \"rust\"\nservers = []\nformatter = { command = \"sh\", args = [\"-c\", \"exit 1\"] }\n",
    )
    .unwrap();
    app.lsp.load(&config, app.events.wake());
    assert!(!app.request_format().unwrap());
    wait_for_formatter(&mut app);
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("fn main() {}\n"));
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("formatter exited")));
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn external_formatter_discards_a_changed_buffer() {
    let (dir, mut app) = app_with_formatter("external_format_stale", "sh");
    let config = dir.join("languages.toml");
    fs::write(
        &config,
        "[[language]]\nname = \"rust\"\nservers = []\nformatter = { command = \"sh\", args = [\"-c\", \"sleep 0.2; tr a-z A-Z\"] }\n",
    )
    .unwrap();
    app.lsp.load(&config, app.events.wake());
    assert!(!app.request_format().unwrap());
    app.editor.apply(Action::EnterInsert);
    app.editor.apply(Action::InsertChar('x'));
    app.editor.apply(Action::ExitInsert);
    wait_for_formatter(&mut app);
    assert_eq!(
        app.editor.buffer_text(1).as_deref(),
        Some("xfn main() {}\n")
    );
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("Buffer changed")));
    let _ = fs::remove_dir_all(dir);
}
