use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEvent};

use crate::recovery::Snapshot;

pub(crate) struct RecoveryPrompt {
    candidates: VecDeque<RecoveryCandidate>,
    total: usize,
    shown: bool,
}

pub(super) struct RecoveryCandidate {
    pub(super) snapshot: Snapshot,
    pub(super) conflict: bool,
}

#[derive(Clone, Copy)]
pub(super) enum RecoveryChoice {
    Recover,
    Discard,
    Skip,
}

impl RecoveryPrompt {
    pub(super) fn new(candidates: Vec<RecoveryCandidate>) -> Option<Self> {
        let total = candidates.len();
        (!candidates.is_empty()).then(|| Self {
            candidates: candidates.into(),
            total,
            shown: false,
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub(super) fn pop(&mut self) -> Option<RecoveryCandidate> {
        self.candidates.pop_front()
    }

    pub(super) fn lines(&self, width: usize) -> Option<Vec<String>> {
        let candidate = self.candidates.front()?;
        let index = self
            .total
            .saturating_sub(self.candidates.len())
            .saturating_add(1);
        let conflict = if candidate.conflict {
            "The file changed on disk after this recovery snapshot."
        } else {
            "Unsaved changes were found."
        };
        let text = format!(
            "Recovery snapshot {index}/{}\n{}\n{conflict}\n\nr recover  d discard  s skip",
            self.total, candidate.snapshot.header.name
        );
        Some(super::super::pager_lines(&text, width))
    }

    pub(super) fn mark_shown(&mut self) {
        self.shown = true;
    }

    pub(super) fn shown(&self) -> bool {
        self.shown
    }

    #[cfg(test)]
    pub(super) fn total(&self) -> usize {
        self.total
    }
}

pub(super) fn recovery_choice(key: KeyEvent) -> Option<RecoveryChoice> {
    match key.code {
        KeyCode::Char('r' | 'R') => Some(RecoveryChoice::Recover),
        KeyCode::Char('d' | 'D') => Some(RecoveryChoice::Discard),
        KeyCode::Char('s' | 'S') => Some(RecoveryChoice::Skip),
        _ => None,
    }
}
