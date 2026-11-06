use std::time::{Duration, Instant};

use super::App;

pub(super) const EDITOR_SYNC_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Default)]
pub(super) struct EditorSync {
    due: Option<Instant>,
}

impl EditorSync {
    pub(super) fn schedule(&mut self, delay: Duration) {
        self.due = Some(Instant::now() + delay);
    }

    pub(super) fn timeout(&self, now: Instant) -> Option<Duration> {
        self.due.map(|due| due.saturating_duration_since(now))
    }

    pub(super) fn take_due(&mut self, now: Instant) -> bool {
        if self.due.is_none_or(|due| due > now) {
            return false;
        }
        self.due = None;
        true
    }
}

impl App {
    pub(super) fn schedule_deferred_editor_sync(&mut self, delay: Duration) {
        self.editor_sync.schedule(delay);
    }

    pub(super) fn deferred_editor_sync_timeout(&self, now: Instant) -> Option<Duration> {
        self.editor_sync.timeout(now)
    }

    pub(super) fn flush_deferred_editor_sync(&mut self, now: Instant) {
        if !self.editor_sync.take_due(now) {
            return;
        }
        if self.git_signs {
            self.sync_git();
        }
        self.sync_merge_conflicts();
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use kawi_buffer::Buffer;
    use kawi_editor::Action;
    use kawi_git::GitFile;

    use crate::app::git::GitSession;
    use crate::keymap::Flavor;

    use super::*;

    #[test]
    fn schedule_sets_timeout_and_take_clears_it() {
        let now = Instant::now();
        let mut sync = EditorSync::default();
        sync.schedule(Duration::from_millis(50));
        assert!(sync.timeout(now).is_some());
        assert!(!sync.take_due(now));
        assert!(sync.take_due(now + Duration::from_millis(100)));
        assert!(sync.timeout(now).is_none());
    }

    #[test]
    fn deferred_git_sync_waits_for_idle() {
        let mut app = App::new(Buffer::from_string("a\nb\n"), Flavor::Vim);
        app.git_signs = true;
        let file = GitFile {
            root: PathBuf::from("/tmp"),
            path: PathBuf::from("/tmp/a"),
            relpath: "a".to_string(),
            mode: "100644".to_string(),
            untracked: false,
            crlf: false,
            no_final_newline: false,
            head_no_final_newline: false,
            base: Vec::new(),
            head: Vec::new(),
        };
        let lines = vec!["a".to_string(), "b".to_string()];
        app.git = Some(GitSession::new(file, &lines, 0, false));
        app.editor.apply(Action::EnterInsert);
        app.editor.apply(Action::InsertChar('x'));
        let revision = app.editor.buffer_revision();
        app.sync_git_deferred();
        assert!(app.editor_sync.timeout(Instant::now()).is_some());
        assert_eq!(app.git.as_ref().map(GitSession::revision), Some(0));
        app.flush_deferred_editor_sync(Instant::now() + Duration::from_millis(200));
        assert_eq!(app.git.as_ref().map(GitSession::revision), Some(revision));
    }
}
