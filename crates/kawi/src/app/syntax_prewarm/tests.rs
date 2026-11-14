use std::fs;

use kawi_buffer::Buffer;

use super::*;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

fn app_with_text(text: &str) -> App {
    App::new(Buffer::from_string(text), Flavor::Vim)
}

#[test]
fn prewarm_starts_syntax_for_the_current_buffer() {
    let dir = temp_dir("startup_syntax");
    let path = dir.join("alpha.rs");
    fs::write(&path, "fn main() {}\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    assert!(app.syntax.is_none());
    app.prewarm_syntax();
    assert!(app.syntax.is_some());
    assert!(!app.syntax_pending);
    assert!(app.syntax_key.is_some());
    assert!(!app.syntax_spans.is_empty());
    drop(app);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn prewarm_skips_buffers_without_syntax() {
    let mut app = app_with_text("plain text\n");
    app.prewarm_syntax();
    assert!(app.syntax.is_none());
}
