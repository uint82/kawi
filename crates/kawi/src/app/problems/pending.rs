use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use kawi_buffer::diff_ropes;
use ropey::Rope;

use super::{EditorText, Problems};

pub(super) struct PendingShift {
    base: Rope,
    current: Rope,
    due: Instant,
}

impl Problems {
    pub(super) fn observe_editor_after(
        &mut self,
        path: Option<&Path>,
        revision: u64,
        rope: Rope,
        delay: Duration,
    ) {
        let Some(path) = path else {
            self.editor = None;
            return;
        };
        if self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.path == path && editor.revision == revision)
        {
            return;
        }
        let previous = self.editor.replace(EditorText {
            path: path.to_path_buf(),
            revision,
            rope,
        });
        if let Some(previous) = previous {
            self.queue_previous_shift(previous, delay);
        }
    }

    fn queue_previous_shift(&mut self, previous: EditorText, delay: Duration) {
        let Some(current) = self.editor.as_ref() else {
            return;
        };
        if previous.path != current.path || previous.revision == current.revision {
            return;
        }
        let path = current.path.clone();
        let old = previous.rope;
        let new = current.rope.clone();
        if delay.is_zero() {
            self.apply_shift(&path, &old, &new);
        } else {
            self.queue_shift(path, old, new, delay);
        }
    }

    fn queue_shift(&mut self, path: PathBuf, base: Rope, current: Rope, delay: Duration) {
        let due = Instant::now() + delay;
        match self.pending_shifts.get_mut(&path) {
            Some(pending) => {
                pending.current = current;
                pending.due = due;
            }
            None => {
                self.pending_shifts
                    .insert(path, PendingShift { base, current, due });
            }
        }
    }

    pub(in crate::app) fn flush_pending_shifts(&mut self, now: Instant) {
        let due = self
            .pending_shifts
            .iter()
            .filter(|(_, pending)| pending.due <= now)
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        for path in due {
            let Some(pending) = self.pending_shifts.remove(&path) else {
                continue;
            };
            self.apply_shift(&path, &pending.base, &pending.current);
        }
    }

    pub(in crate::app) fn shift_wait_timeout(&self, now: Instant) -> Option<Duration> {
        self.pending_shifts
            .values()
            .map(|pending| pending.due.saturating_duration_since(now))
            .min()
    }

    fn apply_shift(&mut self, path: &Path, old: &Rope, new: &Rope) {
        let Some(change) = diff_ropes(old, new) else {
            return;
        };
        self.shift_lsp(path, old, new, change);
        self.invalidate_index();
    }
}
