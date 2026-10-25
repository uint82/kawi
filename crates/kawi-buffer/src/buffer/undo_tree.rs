use crate::history_nav::{changed_cursor, HistoryStep, UndoLeaf, UndoOffset};

use super::Buffer;

impl Buffer {
    pub fn jump_undo_sequence(
        &mut self,
        row: usize,
        col: usize,
        target: u64,
    ) -> Option<(usize, usize)> {
        if self.readonly {
            return None;
        }
        let steps = self.history.path_to(target)?;
        self.apply_history_steps(row, col, &steps)
    }

    pub fn jump_chronological(
        &mut self,
        row: usize,
        col: usize,
        forward: bool,
        count: usize,
    ) -> Option<(usize, usize)> {
        if self.readonly {
            return None;
        }
        let target = self.history.chronological_target(forward, count)?;
        let steps = self.history.path_to(target)?;
        self.apply_history_steps(row, col, &steps)
    }

    pub fn jump_undo_offset(
        &mut self,
        row: usize,
        col: usize,
        offset: UndoOffset,
        forward: bool,
    ) -> Option<(usize, usize)> {
        if self.readonly {
            return None;
        }
        let target = self.history.offset_target(offset, forward)?;
        let steps = self.history.path_to(target)?;
        self.apply_history_steps(row, col, &steps)
    }

    pub fn undo_leaves(&self) -> Vec<UndoLeaf> {
        self.history.leaves()
    }

    fn apply_history_steps(
        &mut self,
        mut row: usize,
        mut col: usize,
        steps: &[HistoryStep],
    ) -> Option<(usize, usize)> {
        if steps.is_empty() {
            return Some((row, col));
        }
        let mut before_last = None;
        for (index, step) in steps.iter().enumerate() {
            if index + 1 == steps.len() {
                before_last = Some(self.rope.clone());
            }
            (row, col) = self.apply_history_step(row, col, *step)?;
        }
        self.touch();
        Some(
            before_last
                .and_then(|before| changed_cursor(&before, &self.rope))
                .unwrap_or((row, col)),
        )
    }

    fn apply_history_step(
        &mut self,
        row: usize,
        col: usize,
        step: HistoryStep,
    ) -> Option<(usize, usize)> {
        let (restored, adjust) = match step {
            HistoryStep::Undo(target) => {
                self.history
                    .undo_to(&mut self.rope, row, col, self.line_changes, target)?
            }
            HistoryStep::Redo(target) => {
                self.history
                    .redo_to(&mut self.rope, row, col, self.line_changes, target)?
            }
        };
        self.push_adjustment(adjust);
        Some(restored)
    }
}
