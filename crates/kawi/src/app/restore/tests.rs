use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_editor::{paths, Action, FoldMethod};
use std::fs;
use std::path::Path;

use crate::keymap::Flavor;
use crate::session::{
    SessionDirection, SessionFold, SessionFoldMethod, SessionView, SessionWindow,
};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_restore_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

fn empty_app() -> App {
    App::new(Buffer::from_string(""), Flavor::Vim)
}

fn view(row: usize, col: usize, top: usize) -> SessionView {
    SessionView {
        row,
        col,
        desired: col,
        top,
        leftcol: 0,
    }
}

fn leaf(id: u64, buffer: usize, row: usize, col: usize) -> SessionWindow {
    SessionWindow::Leaf {
        id,
        buffer,
        view: view(row, col, 0),
    }
}

fn split(first: SessionWindow, second: SessionWindow) -> SessionWindow {
    SessionWindow::Split {
        direction: SessionDirection::Vertical,
        ratio: 40,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn canonical_cwd() -> PathBuf {
    std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap()
}

fn cwd() -> String {
    canonical_cwd().to_string_lossy().into_owned()
}

fn session(buffers: Vec<SessionBuffer>, windows: SessionWindow, active: u64) -> Session {
    Session {
        version: SESSION_VERSION,
        cwd: cwd(),
        buffers,
        windows,
        active_window: active,
    }
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("key");
}

fn create_manual_fold(app: &mut App) {
    app.editor.apply(Action::JumpTo(1, 0));
    app.editor.apply(Action::EnterVisualLine);
    app.editor.apply(Action::JumpTo(3, 0));
    app.editor.apply(Action::FoldCreate);
}

fn create_indent_folds(app: &mut App) {
    app.editor.set_shiftwidth(4);
    app.editor.set_fold_method(FoldMethod::Indent);
    app.editor.close_all_folds();
}

#[test]
fn snapshot_captures_buffers_windows_and_views() {
    let dir = temp_dir("snapshot");
    let alpha = write_file(&dir, "alpha.txt", "alpha\nbeta\ngamma\n");
    let bravo = write_file(&dir, "bravo.txt", "bravo\ncharlie\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&alpha), false).unwrap();
    app.editor.apply(Action::JumpTo(2, 1));
    app.split_window(true, Some(bravo.clone())).unwrap();
    let active = app.windows.active();
    let session = app.session_snapshot().expect("snapshot");
    assert_eq!(session.version, SESSION_VERSION);
    assert_eq!(session.cwd, cwd());
    assert_eq!(session.buffers.len(), 2);
    assert!(session.buffers.iter().all(|buffer| buffer.id.is_some()));
    assert_eq!(
        session.buffers[0].path.as_deref(),
        Some(paths::absolute(&alpha, Path::new(&cwd())).to_str().unwrap())
    );
    assert_eq!(
        session.buffers[1].path.as_deref(),
        Some(paths::absolute(&bravo, Path::new(&cwd())).to_str().unwrap())
    );
    assert_eq!(session.active_window, active);
    match &session.windows {
        SessionWindow::Split { direction, .. } => {
            assert_eq!(*direction, SessionDirection::Vertical);
        }
        SessionWindow::Leaf { .. } => panic!("expected split"),
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn session_restores_manual_folds_when_content_matches() {
    let dir = temp_dir("fold_restore");
    let alpha = write_file(&dir, "alpha.txt", "one\ntwo\nthree\nfour\nfive\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&alpha), false).unwrap();
    create_manual_fold(&mut app);
    let session = app.session_snapshot().expect("snapshot");
    let folds = session.buffers[0].folds.as_ref().expect("folds");
    assert_eq!(folds.method, SessionFoldMethod::Manual);
    assert_eq!(folds.manual, vec![SessionFold { start: 1, end: 3 }]);
    assert_eq!(folds.closed, vec![1]);
    let mut restored = empty_app();
    assert_eq!(restored.restore_session(&session), None);
    assert_eq!(restored.editor.visible_rows(0, 10), vec![0, 1, 4]);
    assert_eq!(restored.editor.fold_marker(1), '+');
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn session_drops_manual_folds_when_content_changed() {
    let dir = temp_dir("fold_changed");
    let alpha = write_file(&dir, "alpha.txt", "one\ntwo\nthree\nfour\nfive\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&alpha), false).unwrap();
    create_manual_fold(&mut app);
    let session = app.session_snapshot().expect("snapshot");
    fs::write(&alpha, "one\ntwo\nthree\nfour\nfive\nsix\n").unwrap();
    let mut restored = empty_app();
    assert_eq!(restored.restore_session(&session), None);
    assert_eq!(restored.editor.visible_rows(0, 10), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(restored.editor.fold_marker(1), ' ');
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn session_keeps_indent_folds_when_content_changes() {
    let dir = temp_dir("fold_indent_changed");
    let alpha = write_file(
        &dir,
        "alpha.py",
        "def main():\n    a = 1\n    b = 2\nd = 4\n",
    );
    let mut app = empty_app();
    app.editor.edit_file(Some(&alpha), false).unwrap();
    create_indent_folds(&mut app);
    let session = app.session_snapshot().expect("snapshot");
    let saved = session.buffers[0].folds.as_ref().expect("folds");
    assert_eq!(saved.method, SessionFoldMethod::Indent);
    assert_eq!(saved.closed, vec![1]);
    fs::write(
        &alpha,
        "def main():\n    a = 1\n    b = 2\n    c = 3\nd = 5\n",
    )
    .unwrap();
    let mut restored = empty_app();
    restored.editor.set_shiftwidth(4);
    restored.editor.set_fold_method(FoldMethod::Indent);
    assert_eq!(restored.restore_session(&session), None);
    assert_eq!(restored.editor.fold_method(), FoldMethod::Indent);
    let after = restored
        .editor
        .buffer_fold_snapshot(restored.editor.buffer_number())
        .expect("restored folds");
    assert_eq!(after.closed, vec![1]);
    assert_eq!(restored.editor.visible_rows(0, 10), vec![0, 1, 4]);
    assert_eq!(restored.editor.fold_marker(1), '+');
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn session_ignores_closed_starts_from_another_fold_method() {
    let dir = temp_dir("fold_method_changed");
    let alpha = write_file(
        &dir,
        "alpha.py",
        "def main():\n    a = 1\n    b = 2\nd = 4\n",
    );
    let mut app = empty_app();
    app.editor.edit_file(Some(&alpha), false).unwrap();
    create_indent_folds(&mut app);
    let session = app.session_snapshot().expect("snapshot");
    let mut restored = empty_app();
    assert_eq!(restored.restore_session(&session), None);
    assert_eq!(restored.editor.visible_rows(0, 10), vec![0, 1, 2, 3]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn restore_rebuilds_windows_and_per_window_views() {
    let dir = temp_dir("restore");
    let alpha = write_file(&dir, "alpha.txt", "alpha\nbeta\ngamma\n");
    let bravo = write_file(&dir, "bravo.txt", "bravo\ncharlie\n");
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha.clone()),
                id: None,
                folds: None,
            },
            SessionBuffer {
                path: Some(bravo.clone()),
                id: None,
                folds: None,
            },
        ],
        split(leaf(1, 0, 2, 3), leaf(2, 1, 1, 2)),
        2,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    assert_eq!(app.windows.len(), 2);
    assert_eq!(app.editor.filename(), bravo);
    assert_eq!(app.cursor_position(), (1, 2));
    assert_eq!(app.editor.buffer_numbers().len(), 2);
    assert!(app.focus_window_direction(super::super::window::FocusDirection::Left));
    assert_eq!(app.editor.filename(), alpha);
    assert_eq!(app.cursor_position(), (2, 3));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn restore_skips_missing_buffers_and_repairs_focus() {
    let dir = temp_dir("missing");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let missing = dir.join("missing.txt").to_string_lossy().into_owned();
    let session = session(
        vec![
            SessionBuffer {
                path: Some(missing),
                id: None,
                folds: None,
            },
            SessionBuffer {
                path: Some(alpha.clone()),
                id: None,
                folds: None,
            },
        ],
        split(leaf(1, 0, 0, 0), leaf(2, 1, 0, 0)),
        1,
    );
    let mut app = empty_app();
    assert_eq!(
        app.restore_session(&session),
        Some("session: skipped 1 unavailable buffer".to_string())
    );
    assert_eq!(app.windows.len(), 1);
    assert_eq!(app.editor.filename(), alpha);
    assert_eq!(app.windows.active(), 2);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn restore_clamps_out_of_range_views() {
    let dir = temp_dir("clamp_views");
    let alpha = write_file(&dir, "alpha.txt", "alpha\nbeta\n");
    let bravo = write_file(&dir, "bravo.txt", "one\n");
    let wild = SessionView {
        row: 20,
        col: 20,
        desired: 20,
        top: 20,
        leftcol: 20,
    };
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha.clone()),
                id: None,
                folds: None,
            },
            SessionBuffer {
                path: Some(bravo.clone()),
                id: None,
                folds: None,
            },
        ],
        split(
            SessionWindow::Leaf {
                id: 1,
                buffer: 0,
                view: wild,
            },
            SessionWindow::Leaf {
                id: 2,
                buffer: 1,
                view: wild,
            },
        ),
        1,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    assert_eq!(app.cursor_position(), (1, 3));
    assert_eq!(app.editor.view_top(), 1);
    assert_eq!(app.editor.view_leftcol(), 3);
    assert!(app.focus_window_direction(super::super::window::FocusDirection::Right));
    assert_eq!(app.editor.filename(), bravo);
    assert_eq!(app.cursor_position(), (0, 2));
    assert_eq!(app.editor.view_top(), 0);
    assert_eq!(app.editor.view_leftcol(), 2);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn restore_collapses_nested_missing_leaves() {
    let dir = temp_dir("nested_missing");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let bravo = write_file(&dir, "bravo.txt", "bravo\n");
    let missing = dir.join("missing.txt").to_string_lossy().into_owned();
    let nested = SessionWindow::Split {
        direction: SessionDirection::Horizontal,
        ratio: 50,
        first: Box::new(leaf(2, 1, 0, 0)),
        second: Box::new(leaf(3, 2, 0, 0)),
    };
    let windows = SessionWindow::Split {
        direction: SessionDirection::Vertical,
        ratio: 50,
        first: Box::new(leaf(1, 0, 0, 0)),
        second: Box::new(nested),
    };
    let session = session(
        vec![
            SessionBuffer {
                path: Some(alpha.clone()),
                id: None,
                folds: None,
            },
            SessionBuffer {
                path: Some(missing),
                id: None,
                folds: None,
            },
            SessionBuffer {
                path: Some(bravo),
                id: None,
                folds: None,
            },
        ],
        windows,
        2,
    );
    let mut app = empty_app();
    assert_eq!(
        app.restore_session(&session),
        Some("session: skipped 1 unavailable buffer".to_string())
    );
    assert_eq!(app.windows.len(), 2);
    assert_eq!(app.windows.active(), 1);
    assert_eq!(app.editor.filename(), alpha);
    let layout = app.window_layout();
    assert_eq!(layout.panes.len(), 2);
    assert_eq!(layout.separators.len(), 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn restore_adopts_stored_buffer_ids() {
    let dir = temp_dir("buffer_ids");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let session = session(
        vec![SessionBuffer {
            path: Some(alpha),
            id: Some("stable-id".to_string()),
            folds: None,
        }],
        leaf(1, 0, 0, 0),
        1,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    assert_eq!(
        app.editor.buffer_id(app.editor.buffer_number()),
        Some("stable-id")
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn restore_keeps_an_empty_unnamed_buffer() {
    let dir = temp_dir("unnamed");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let session = session(
        vec![
            SessionBuffer {
                path: None,
                id: None,
                folds: None,
            },
            SessionBuffer {
                path: Some(alpha.clone()),
                id: None,
                folds: None,
            },
        ],
        split(leaf(1, 0, 0, 0), leaf(2, 1, 0, 0)),
        1,
    );
    let mut app = empty_app();
    assert_eq!(app.restore_session(&session), None);
    assert_eq!(app.windows.len(), 2);
    assert_eq!(app.editor.filename(), "[No Name]");
    assert_eq!(app.editor.buffer_numbers().len(), 2);
    assert_eq!(app.windows.count_buffer(app.editor.buffer_number()), 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn snapshot_saves_dirty_workspaces() {
    let dir = temp_dir("dirty");
    let alpha = write_file(&dir, "alpha.txt", "alpha\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&alpha), false).unwrap();
    press(&mut app, KeyCode::Char('x'));
    let session = app.session_snapshot().expect("snapshot");
    assert_eq!(session.buffers.len(), 1);
    assert_eq!(
        session.buffers[0].path.as_deref(),
        Some(paths::absolute(&alpha, &canonical_cwd()).to_str().unwrap())
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn session_file_round_trips_through_restore() {
    let dir = temp_dir("roundtrip");
    let alpha = write_file(&dir, "alpha.txt", "alpha\nbeta\n");
    let bravo = write_file(&dir, "bravo.txt", "bravo\n");
    let mut app = empty_app();
    app.editor.edit_file(Some(&alpha), false).unwrap();
    app.editor.apply(Action::JumpTo(1, 2));
    app.split_window(true, Some(bravo.clone())).unwrap();
    let session = app.session_snapshot().expect("snapshot");
    let sessions = dir.join("sessions");
    let cwd = canonical_cwd();
    crate::session::save_for_cwd(&sessions, &cwd, &session).unwrap();
    let loaded = crate::session::load_for_cwd(&sessions, &cwd)
        .unwrap()
        .expect("session");
    let mut restored = empty_app();
    assert_eq!(restored.restore_session(&loaded), None);
    assert_eq!(restored.windows.len(), 2);
    assert_eq!(restored.editor.filename(), bravo);
    assert!(restored.focus_window_direction(super::super::window::FocusDirection::Left));
    assert_eq!(restored.editor.filename(), alpha);
    assert_eq!(restored.cursor_position(), (1, 2));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn restore_with_only_missing_buffers_falls_back_to_one_window() {
    let dir = temp_dir("all_missing");
    let missing = dir.join("missing.txt").to_string_lossy().into_owned();
    let session = session(
        vec![SessionBuffer {
            path: Some(missing),
            id: None,
            folds: None,
        }],
        leaf(1, 0, 0, 0),
        1,
    );
    let mut app = empty_app();
    let warning = app.restore_session(&session).expect("warning");
    assert!(warning.contains("skipped 1 unavailable buffer"));
    assert!(warning.contains("no windows could be restored"));
    assert_eq!(app.windows.len(), 1);
    assert_eq!(app.editor.filename(), "[No Name]");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn snapshot_saves_an_empty_workspace() {
    let mut app = empty_app();
    let session = app.session_snapshot().expect("snapshot");
    assert_eq!(session.buffers.len(), 1);
    assert_eq!(session.buffers[0].path, None);
    assert!(matches!(session.windows, SessionWindow::Leaf { .. }));
}
