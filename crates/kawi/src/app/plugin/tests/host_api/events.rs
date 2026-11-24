use std::fs;
use std::path::Path;

use kawi_buffer::Buffer;
use kawi_lsp::DiagnosticSeverity;
use serde_json::Value;

use crate::app::App;
use crate::keymap::Flavor;
use crate::test_support::diagnostic;

use super::super::common::temp_dir;
use super::app;

#[test]
fn tracks_and_deduplicates_editor_events() {
    let mut app = app();
    app.plugins.test_events.clear();
    sync(&mut app, Some((1, 1)));
    assert!(new_events(&app).is_empty(), "{:?}", app.plugins.test_events);

    app.editor.set_cursor_position(0, 1);
    app.plugins.test_events.clear();
    sync(&mut app, Some((1, 1)));
    let events = new_events(&app);
    assert_eq!(events, vec!["cursor_moved"]);

    app.editor.set_selection_ends((0, 0), (0, 2));
    app.plugins.test_events.clear();
    sync(&mut app, Some((1, 1)));
    let events = new_events(&app);
    assert!(events.contains(&"selection_changed"));
    assert!(events.contains(&"cursor_moved"));

    app.plugins.test_events.clear();
    sync(&mut app, Some((1, 1)));
    assert!(new_events(&app).is_empty());

    app.plugins.test_events.clear();
    sync(&mut app, None);
    assert_eq!(new_events(&app), vec!["focus_lost"]);

    app.plugins.test_events.clear();
    sync(&mut app, Some((1, 1)));
    assert_eq!(new_events(&app), vec!["focus_gained"]);

    app.plugins.last_active_buffer = Some(99);
    app.plugins.last_active_window = Some((9, 99));
    app.plugins.test_events.clear();
    sync(&mut app, Some((1, 1)));
    let events = new_events(&app);
    assert!(events.contains(&"buffer_leave"));
    assert!(events.contains(&"buffer_enter"));
    assert!(events.contains(&"window_leave"));
    assert!(events.contains(&"window_enter"));
}

#[test]
fn diagnostics_changed_only_reports_buffers_whose_diagnostics_changed() {
    let (mut app, root, second) = create_two_file_app("diagnostics_changed_precision");
    sync(&mut app, Some((1, 1)));
    app.problems.replace_lsp_test(
        "test",
        Path::new(&second),
        vec![diagnostic(0, DiagnosticSeverity::Warning, "check")],
    );
    app.plugins.test_events.clear();
    sync(&mut app, Some((1, 1)));
    let events = diagnostics_events(&app);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0]["path"].as_str(), Some(second.as_str()));
    assert_eq!(
        events[0]["buffer"].as_u64(),
        Some(app.editor.buffer_number() as u64)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unchanged_diagnostics_do_not_repeat_after_an_unrelated_revision() {
    let (mut app, root, second) = create_two_file_app("diagnostics_changed_unrelated");
    sync(&mut app, Some((1, 1)));
    app.problems.replace_lsp_test(
        "test",
        Path::new(&second),
        vec![diagnostic(0, DiagnosticSeverity::Warning, "check")],
    );
    sync(&mut app, Some((1, 1)));
    app.plugins.test_events.clear();
    app.problems.clear_source("unrelated");
    sync(&mut app, Some((1, 1)));
    let events = diagnostics_events(&app);
    assert!(events.is_empty(), "{events:?}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn diagnostics_changed_reports_cleared_diagnostics_once() {
    let (mut app, root, second) = create_two_file_app("diagnostics_changed_cleared");
    sync(&mut app, Some((1, 1)));
    app.problems.replace_lsp_test(
        "test",
        Path::new(&second),
        vec![diagnostic(0, DiagnosticSeverity::Warning, "check")],
    );
    sync(&mut app, Some((1, 1)));
    app.plugins.test_events.clear();
    app.problems
        .replace_lsp_test("test", Path::new(&second), Vec::new());
    sync(&mut app, Some((1, 1)));
    let events = diagnostics_events(&app);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0]["path"].as_str(), Some(second.as_str()));
    assert!(events[0]["diagnostics"].as_array().unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}

fn create_two_file_app(name: &str) -> (App, std::path::PathBuf, String) {
    let root = temp_dir(name);
    let first = root.join("first.txt");
    let second = root.join("second.txt");
    fs::write(&first, "alpha\n").unwrap();
    fs::write(&second, "beta\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(first.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    app.editor
        .open_file_buffer(second.to_str().unwrap())
        .unwrap();
    (app, root, second.to_str().unwrap().to_string())
}

fn diagnostics_events(app: &App) -> Vec<&Value> {
    app.plugins
        .test_events
        .iter()
        .filter(|(event, _)| event == "diagnostics_changed")
        .map(|(_, payload)| payload)
        .collect()
}

fn sync(app: &mut App, focus: Option<(u64, u64)>) {
    app.plugins
        .sync_plugin_state(&mut app.editor, &mut app.problems, &app.windows, focus);
}

fn new_events(app: &App) -> Vec<&str> {
    const NEW_EVENTS: &[&str] = &[
        "buffer_enter",
        "buffer_leave",
        "window_enter",
        "window_leave",
        "cursor_moved",
        "selection_changed",
        "focus_gained",
        "focus_lost",
    ];
    app.plugins
        .test_events
        .iter()
        .map(|(event, _)| event.as_str())
        .filter(|event| NEW_EVENTS.contains(event))
        .collect()
}
