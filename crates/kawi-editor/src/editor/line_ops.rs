use super::Editor;

impl Editor {
    pub(super) fn delete_touched_lines(&mut self) {
        let (first, last) = self.touched_rows();
        let col = self.cursor.col();
        self.remove_lines(first, last);
        let row = first.min(self.buffer.line_count().saturating_sub(1));
        let max = self.buffer.line_len(row).saturating_sub(1);
        let col = col.min(max);
        self.collapse_to(row, col);
        self.cursor.set_position(row, col);
    }

    pub(super) fn yank_touched_lines(&mut self) {
        let (first, last) = self.touched_rows();
        self.yank_line_range(first, last);
        self.cursor.set_position(first, 0);
    }

    pub(super) fn change_touched_lines(&mut self) {
        let (first, last) = self.touched_rows();
        self.exit_visual();
        self.change_lines(first, last);
    }

    fn touched_rows(&self) -> (usize, usize) {
        let ((first, _), (last, _)) = self.operator_range();
        (first, last)
    }
}
