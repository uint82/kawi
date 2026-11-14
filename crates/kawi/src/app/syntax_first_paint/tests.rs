use std::fs;
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;

use super::*;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

fn rust_app(dir: &std::path::Path, name: &str, text: &str) -> App {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    )
}

#[test]
fn small_new_documents_wait_for_their_first_paint() {
    let dir = temp_dir("first_paint_small");
    let mut app = rust_app(&dir, "first.rs", "fn first() {}\n");
    app.refresh_syntax();
    let first_key = app.syntax_key.expect("first syntax key");
    assert!(app.wait_for_syntax_until(first_key, Instant::now() + Duration::from_secs(5)));

    let second = dir.join("second.rs");
    fs::write(&second, "fn second() {}\n").unwrap();
    app.editor
        .open_file_buffer(second.to_str().unwrap())
        .expect("second buffer");

    assert!(app.refresh_syntax());
    let key = app.syntax_key.expect("new syntax key");
    assert!(app.wait_for_syntax_until(key, Instant::now() + Duration::from_secs(5)));
    assert_eq!(
        app.syntax_spans_key.map(|applied| applied.document),
        Some(app.editor.buffer_number() as u64)
    );
    assert!(!app.syntax_spans.is_empty());
    let layout = app.window_layout();
    let panes = app.frame_panes(&layout);
    assert!(panes
        .iter()
        .any(|pane| pane.active && !pane.syntax_spans.is_empty()));
    assert!(!app.refresh_syntax());

    drop(app);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn large_documents_keep_the_async_path() {
    let line = "// pad\n";
    let text = line.repeat(FIRST_PAINT_MAX_BYTES / line.len() + 1);
    let mut buffer = Buffer::from_string(&text);
    buffer.path = Some("large.rs".to_string());
    let mut app = App::new(buffer, Flavor::Vim);

    assert!(!app.refresh_syntax());
    assert!(app.syntax_pending);
    let key = app.syntax_key.expect("large syntax key");
    assert!(app.wait_for_syntax_until(key, Instant::now() + Duration::from_secs(5)));
    assert!(!app.syntax_spans.is_empty());
}

#[test]
fn first_paint_size_limit_includes_the_boundary() {
    assert!(fits_first_paint(FIRST_PAINT_MAX_BYTES));
    assert!(!fits_first_paint(FIRST_PAINT_MAX_BYTES + 1));
}
