use std::ffi::OsString;
use std::path::Path;

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

pub(super) struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    pub(super) fn set(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
