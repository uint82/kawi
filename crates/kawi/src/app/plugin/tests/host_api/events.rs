use crate::app::App;

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
