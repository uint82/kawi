use kawi_buffer::{UndoLeaf, UndoOffset};

use super::Editor;

impl Editor {
    pub fn chronological_undo(&mut self, forward: bool) -> bool {
        let count = self.take_count();
        let before = (self.cursor.row(), self.cursor.col());
        let Some((row, col)) = self
            .buffer
            .jump_chronological(before.0, before.1, forward, count)
        else {
            return false;
        };
        self.cursor.set_position(row, col);
        self.set_pcmark_at(before);
        self.apply_line_adjustments();
        self.remember_wanted_column();
        true
    }

    pub fn jump_undo_sequence(&mut self, sequence: u64) -> bool {
        let before = (self.cursor.row(), self.cursor.col());
        let Some((row, col)) = self.buffer.jump_undo_sequence(before.0, before.1, sequence) else {
            return false;
        };
        self.cursor.set_position(row, col);
        self.set_pcmark_at(before);
        self.apply_line_adjustments();
        self.remember_wanted_column();
        true
    }

    pub fn jump_undo_offset(&mut self, offset: UndoOffset, forward: bool) -> bool {
        let before = (self.cursor.row(), self.cursor.col());
        let Some((row, col)) = self
            .buffer
            .jump_undo_offset(before.0, before.1, offset, forward)
        else {
            return false;
        };
        self.cursor.set_position(row, col);
        self.set_pcmark_at(before);
        self.apply_line_adjustments();
        self.remember_wanted_column();
        true
    }

    pub fn undo_leaves(&self) -> Vec<UndoLeaf> {
        self.buffer.undo_leaves()
    }
}
