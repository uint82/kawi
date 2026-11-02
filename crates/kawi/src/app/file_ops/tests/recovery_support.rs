use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;

use super::*;
use crate::app::App;
use crate::recovery::JournalPath;

pub(super) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(super) fn journal_path(path: PathBuf) -> JournalPath {
    JournalPath::from(path)
}

pub(super) fn start_app(root: &Path, recovery: &Path) -> App {
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    assert_eq!(app.start_recovery(recovery, root, false), None);
    app
}
