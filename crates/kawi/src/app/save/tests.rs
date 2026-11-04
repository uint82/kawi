use std::fs;
use std::path::Path;

use kawi_buffer::{content_hash, Buffer};
use kawi_editor::{Action, WriteOutcome};
use kawi_lsp::{DocumentEdit, FormattingEvent, TextChange, WorkspaceEdit};

use super::*;
use crate::keymap::Flavor;

fn app_with(name: &str, text: &str) -> (PathBuf, PathBuf, App) {
    let dir = crate::test_support::temp_dir(name);
    let path = dir.join("a.txt");
    fs::write(&path, text).expect("write");
    let app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    (dir, path, app)
}

fn message(app: &App) -> String {
    app.last_message()
        .map(str::to_string)
        .or_else(|| app.pager_lines().map(|lines| lines.join(" ")))
        .unwrap_or_default()
}

fn edit(app: &mut App, text: &str) {
    app.editor.apply(Action::EnterInsert);
    for ch in text.chars() {
        app.editor.apply(Action::InsertChar(ch));
    }
    app.editor.apply(Action::ExitInsert);
}

fn pending(app: &mut App, request: u64, path: &Path) {
    app.pending_save = Some(PendingSave {
        format: SaveFormat::Lsp(request),
        number: 1,
        force: false,
        after: SaveAfter::None,
        visible: true,
        started: Instant::now(),
    });
    assert!(path.exists());
}

fn formatting_event(request: u64, path: &Path, original: &str) -> FormattingEvent {
    FormattingEvent {
        request,
        edit: WorkspaceEdit {
            documents: vec![DocumentEdit {
                path: path.to_path_buf(),
                original_hash: content_hash(original.as_bytes()),
                edits: vec![TextChange {
                    start: 0,
                    end: original.len(),
                    text: "formatted\n".to_string(),
                }],
            }],
        },
    }
}

#[test]
fn saves_directly_when_formatting_is_disabled() {
    let (dir, path, mut app) = app_with("direct", "alpha\n");
    edit(&mut app, "x");
    app.request_save(false, SaveAfter::None, true);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert_eq!(
        app.editor.buffer_text(1).as_deref(),
        Some("xalpha\n"),
        "buffer text"
    );
    assert_eq!(
        app.editor.buffer_is_dirty(1),
        Some(false),
        "buffer should be clean"
    );
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn missing_formatter_falls_back_to_a_direct_save() {
    let (dir, path, mut app) = app_with("no_formatter", "alpha\n");
    app.format_on_save = true;
    edit(&mut app, "x");
    app.request_save(false, SaveAfter::None, true);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert!(app.pending_save.is_none());
    fs::remove_dir_all(dir).expect("cleanup");
}

#[cfg(unix)]
fn load_external_formatter(app: &mut App, dir: &Path, command: &str, args: &str) {
    let config = dir.join("languages.toml");
    fs::write(
        &config,
        format!(
            "[[language]]\nname = \"text\"\nextensions = [\"txt\"]\nservers = []\nformatter = {{ command = {command:?}, args = {args} }}\n"
        ),
    )
    .unwrap();
    app.lsp.load(&config, app.events.wake());
}

fn wait_for_external_save(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && app.pending_save.is_some() {
        app.poll_formatter();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.pending_save.is_none(), "save timed out");
}

#[cfg(unix)]
#[test]
fn external_formatter_runs_before_save() {
    let (dir, path, mut app) = app_with("external_save", "alpha\n");
    app.format_on_save = true;
    edit(&mut app, "x");
    load_external_formatter(&mut app, &dir, "sh", "[\"-c\", \"tr a-z A-Z\"]");
    assert!(!app.request_save(false, SaveAfter::None, true));
    assert!(app.pending_save.is_some());
    assert_eq!(fs::read_to_string(&path).expect("disk"), "alpha\n");
    wait_for_external_save(&mut app);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "XALPHA\n");
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("XALPHA\n"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn external_formatter_failure_saves_unformatted() {
    let (dir, path, mut app) = app_with("external_save_failure", "alpha\n");
    app.format_on_save = true;
    edit(&mut app, "x");
    load_external_formatter(
        &mut app,
        &dir,
        "sh",
        "[\"-c\", \"printf boom >&2; exit 1\"]",
    );
    assert!(!app.request_save(false, SaveAfter::None, true));
    wait_for_external_save(&mut app);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert!(message(&app).contains("saved without formatting"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn external_formatter_saves_a_changed_buffer_unformatted() {
    let (dir, path, mut app) = app_with("external_save_stale", "alpha\n");
    app.format_on_save = true;
    edit(&mut app, "x");
    load_external_formatter(&mut app, &dir, "sh", "[\"-c\", \"sleep 0.2; tr a-z A-Z\"]");
    assert!(!app.request_save(false, SaveAfter::None, true));
    edit(&mut app, "y");
    wait_for_external_save(&mut app);
    assert_eq!(fs::read_to_string(&path).expect("disk"), "yxalpha\n");
    assert!(message(&app).contains("saved without formatting"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn formatted_save_applies_edits_before_writing() {
    let (dir, path, mut app) = app_with("formatted", "alpha\n");
    edit(&mut app, "x");
    pending(&mut app, 7, &path);
    app.finish_formatted_save(formatting_event(7, &path, "xalpha\n"));
    assert_eq!(fs::read_to_string(&path).expect("disk"), "formatted\n");
    assert!(!fs::read_to_string(&path).expect("disk").contains("xalpha"));
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("formatted\n"));
    assert_eq!(app.editor.buffer_is_dirty(1), Some(false));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn formatting_is_one_undoable_step() {
    let (dir, path, mut app) = app_with("undo", "alpha\n");
    edit(&mut app, "x");
    pending(&mut app, 12, &path);
    app.finish_formatted_save(formatting_event(12, &path, "xalpha\n"));
    assert!(app.editor.try_undo());
    assert_eq!(app.editor.buffer_text(1).as_deref(), Some("xalpha\n"));
    assert!(app.editor.is_dirty());
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn write_quit_requests_quit_after_saving() {
    let (dir, path, mut app) = app_with("wq", "alpha\n");
    edit(&mut app, "x");
    app.handle_write_quit(false).expect("write quit");
    assert!(app.quit_pending);
    assert_eq!(fs::read_to_string(path).expect("disk"), "xalpha\n");
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn stale_formatting_saves_the_current_buffer_unformatted() {
    let (dir, path, mut app) = app_with("stale", "alpha\n");
    edit(&mut app, "x");
    pending(&mut app, 8, &path);
    app.finish_formatted_save(formatting_event(8, &path, "different\n"));
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert!(message(&app).contains("saved without formatting"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn formatter_timeout_saves_unformatted() {
    let (dir, path, mut app) = app_with("timeout", "alpha\n");
    edit(&mut app, "x");
    pending(&mut app, 9, &path);
    app.pending_save.as_mut().expect("pending").started = Instant::now() - FORMAT_TIMEOUT;
    assert!(app.save_tick());
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert!(message(&app).contains("formatter timed out"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn formatter_failure_saves_unformatted() {
    let (dir, path, mut app) = app_with("failure", "alpha\n");
    edit(&mut app, "x");
    pending(&mut app, 10, &path);
    app.finish_formatting_failure(10, "server crashed".to_string());
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert!(message(&app).contains("server crashed"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn no_formatting_changes_saves_normally() {
    let (dir, path, mut app) = app_with("no_changes", "alpha\n");
    edit(&mut app, "x");
    pending(&mut app, 11, &path);
    app.finish_formatting_failure(11, "No formatting changes".to_string());
    assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
    assert!(!message(&app).contains("without formatting"));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn report_shape_matches_editor_output() {
    assert_eq!(
        super::super::written_report(
            "notes.txt",
            WriteOutcome {
                lines: 2,
                bytes: 4,
                created: false,
            },
        ),
        "\"notes.txt\" 2L, 4B written"
    );
}
