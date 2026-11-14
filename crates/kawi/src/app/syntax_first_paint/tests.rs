use std::fs;

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
fn small_new_documents_paint_their_first_frame() {
    let dir = temp_dir("first_paint_small");
    let mut app = rust_app(&dir, "first.rs", "fn first() {}\n");
    app.refresh_syntax();
    let first_key = app.syntax_key.expect("first syntax key");
    assert!(app.syntax_spans_cover(&first_key));

    let second = dir.join("second.rs");
    fs::write(&second, "fn second() {}\n").unwrap();
    app.editor
        .open_file_buffer(second.to_str().unwrap())
        .expect("second buffer");

    assert!(app.refresh_syntax());
    let key = app.syntax_key.expect("new syntax key");
    assert!(app.syntax_spans_cover(&key));
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
fn large_documents_paint_a_bounded_first_window() {
    let text = "// pad\n".repeat(20_000);
    let mut buffer = Buffer::from_string(&text);
    buffer.path = Some("large.rs".to_string());
    let mut app = App::new(buffer, Flavor::Vim);

    assert!(app.refresh_syntax());
    assert!(!app.syntax_pending);
    let key = app.syntax_key.expect("large syntax key");
    assert!(app.syntax_spans_cover(&key));
    assert!(!app.syntax_spans.is_empty());
    assert!(app.syntax_window.end < text.lines().count());
}

#[test]
fn cold_typescript_paints_before_the_first_frame() {
    let dir = temp_dir("first_paint_tsx");
    let path = dir.join("view.tsx");
    fs::write(
        &path,
        "export function View() {\n  return <main>Hello</main>;\n}\n",
    )
    .unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );

    assert!(app.refresh_syntax());
    assert!(!app.syntax_pending);
    assert!(!app.syntax_spans.is_empty());
    assert_eq!(
        app.syntax_spans_key.map(|key| key.language),
        Some(kawi_syntax::LanguageId::Tsx)
    );

    drop(app);
    fs::remove_dir_all(dir).unwrap();
}
