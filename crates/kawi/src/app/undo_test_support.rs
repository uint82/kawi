use std::fs;

use kawi_buffer::Buffer;
use kawi_editor::Action;

use super::save::SaveAfter;
use super::App;
use crate::keymap::Flavor;

pub(crate) fn app_with(name: &str, text: &str) -> (std::path::PathBuf, App) {
    let dir = crate::test_support::temp_dir(name);
    let path = dir.join("a.txt");
    fs::write(&path, text).expect("write");
    let app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    (dir, app)
}

pub(crate) fn configured_app(path: &std::path::Path, store: &std::path::Path) -> App {
    let mut app = App::new(
        Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
        Flavor::Vim,
    );
    app.undo.configure(store.to_path_buf());
    app.undo.set_exclusions(Vec::new());
    app.sync_undo_histories();
    app
}

pub(crate) fn edit(app: &mut App, text: &str) {
    app.editor.apply(Action::EnterInsert);
    for ch in text.chars() {
        app.editor.apply(Action::InsertChar(ch));
    }
    app.editor.apply(Action::ExitInsert);
}

pub(crate) fn save(app: &mut App) {
    app.request_save(false, SaveAfter::None, false);
    assert!(app.pending_save.is_none());
}

pub(crate) fn text(app: &App) -> String {
    app.editor.buffer_text(1).expect("buffer")
}

pub(crate) fn branch(app: &mut App) -> String {
    edit(app, "a");
    edit(app, "b");
    let old_branch = text(app);
    app.editor.apply(Action::Undo);
    edit(app, "c");
    old_branch
}
