use super::Editor;
use crate::folds::FoldMethod;

mod buffer;

#[derive(Default)]
pub struct VisibleWindow {
    pub rows: Vec<usize>,
    pub lines: Vec<String>,
}

impl Editor {
    pub fn fold_method(&self) -> FoldMethod {
        self.fold_method
    }

    pub fn set_fold_method(&mut self, method: FoldMethod) {
        self.fold_method = method;
        self.folds.set_method(method);
        self.buffers.set_fold_method(method);
        self.refresh_folds();
        self.normalize_view();
    }

    pub fn set_fold_column(&mut self, column: usize) {
        self.fold_column = column;
        self.folds.set_column(column);
        self.buffers.set_fold_column(column);
    }

    pub fn fold_column(&self) -> usize {
        self.fold_column
    }

    pub fn fold_column_width(&self) -> usize {
        self.folds.column_width()
    }

    pub fn refresh_fold_state(&mut self) {
        self.refresh_folds();
        self.folds.revalidate(self.buffer.line_count());
        self.normalize_view();
    }

    pub fn visible_top(&self) -> usize {
        self.folds
            .visible_top(self.viewport.top(), self.buffer.line_count())
    }

    pub fn visible_rows(&self, top: usize, height: usize) -> Vec<usize> {
        self.folds
            .visible_rows(top, height, self.buffer.line_count())
    }

    pub fn visible_window(&self, top: usize, height: usize) -> VisibleWindow {
        let rows = self.visible_rows(top, height);
        let mut lines: Vec<String> = rows
            .iter()
            .map(|row| self.buffer.line(*row).unwrap_or_default())
            .collect();
        self.apply_command_preview(&rows, &mut lines);
        VisibleWindow { rows, lines }
    }

    pub fn visible_line_count(&self) -> usize {
        self.folds.visible_line_count(self.buffer.line_count())
    }

    pub fn display_row(&self, row: usize) -> usize {
        self.folds.display_row(row, self.buffer.line_count())
    }

    pub fn buffer_row_at_display(&self, display: usize) -> usize {
        self.folds.buffer_row(display, self.buffer.line_count())
    }

    pub fn next_visible_row(&self, row: usize) -> usize {
        let line_count = self.buffer.line_count();
        let next = self.folds.next_visible(row.saturating_add(1), line_count);
        if next < line_count {
            next
        } else {
            self.folds
                .prev_visible(line_count.saturating_sub(1), line_count)
        }
    }

    pub fn previous_visible_row(&self, row: usize) -> usize {
        self.folds
            .prev_visible(row.saturating_sub(1), self.buffer.line_count())
    }

    pub fn visible_row_after(&self, row: usize, count: usize) -> usize {
        let mut row = row;
        for _ in 0..count {
            let next = self.next_visible_row(row);
            if next == row {
                break;
            }
            row = next;
        }
        row
    }

    pub fn visible_row_before(&self, row: usize, count: usize) -> usize {
        let mut row = row;
        for _ in 0..count {
            let previous = self.previous_visible_row(row);
            if previous == row {
                break;
            }
            row = previous;
        }
        row
    }

    pub fn max_view_top(&self) -> usize {
        self.folds
            .max_top(self.buffer.line_count(), self.view_height())
    }

    pub fn visible_bottom(&self) -> usize {
        self.visible_rows(self.visible_top(), self.view_height())
            .last()
            .copied()
            .unwrap_or_else(|| self.visible_top())
    }

    pub fn cursor_screen_row(&self) -> Option<usize> {
        let row = self.cursor.row();
        if self.folds.is_hidden(row) {
            return None;
        }
        Some(
            self.display_row(row)
                .saturating_sub(self.display_row(self.visible_top())),
        )
    }

    pub fn buffer_row_at_screen(&self, screen_row: usize) -> Option<usize> {
        self.visible_rows(self.visible_top(), screen_row.saturating_add(1))
            .get(screen_row)
            .copied()
    }

    pub fn fold_marker(&self, row: usize) -> char {
        if self.fold_column_width() == 0 {
            return ' ';
        }
        self.folds.marker(row)
    }

    pub fn toggle_fold(&mut self) {
        let row = self.cursor.row();
        if let Some(target) = self.folds.toggle_at(row) {
            self.cursor.set_row(target);
            self.apply_desired_column();
        }
        self.normalize_view();
    }

    pub fn open_fold(&mut self) {
        let row = self.cursor.row();
        if let Some(target) = self.folds.open_at(row) {
            self.cursor.set_row(target);
            self.apply_desired_column();
        }
    }

    pub fn close_fold(&mut self) {
        let row = self.cursor.row();
        if let Some(target) = self.folds.close_at(row) {
            self.cursor.set_row(target);
            self.apply_desired_column();
        }
        self.normalize_view();
    }

    pub fn open_all_folds(&mut self) {
        self.folds.open_all();
    }

    fn reveal_folds_at(&mut self, row: usize) {
        while let Some(fold) = self.folds.fold_at(row) {
            if !self.folds.is_closed(fold.start) {
                break;
            }
            self.folds.open_at(row);
        }
    }

    pub fn close_all_folds(&mut self) {
        let row = self.cursor.row();
        self.folds.close_all();
        if self.folds.is_hidden(row) {
            let target = self.folds.prev_visible(row, self.buffer.line_count());
            self.cursor.set_row(target);
            self.apply_desired_column();
        }
        self.normalize_view();
    }

    pub fn next_fold(&mut self) {
        if let Some(start) = self
            .folds
            .next_fold_start(self.cursor.row(), self.buffer.line_count())
        {
            self.cursor.set_row(start);
            self.apply_desired_column();
            self.normalize_view();
        }
    }

    pub fn previous_fold(&mut self) {
        if let Some(start) = self.folds.prev_fold_start(self.cursor.row()) {
            self.cursor.set_row(start);
            self.apply_desired_column();
            self.normalize_view();
        }
    }

    pub fn delete_fold(&mut self) {
        let row = self.cursor.row();
        if let Some(target) = self.folds.delete_at(row) {
            self.cursor.set_row(target);
            self.apply_desired_column();
        }
        self.normalize_view();
    }

    pub fn create_fold(&mut self) {
        if self.fold_method() != FoldMethod::Manual {
            return;
        }
        let Some(((first, _), (last, _))) = self.selection_range() else {
            return;
        };
        let (start, end) = if first <= last {
            (first, last)
        } else {
            (last, first)
        };
        self.folds
            .create_manual(start, end, self.buffer.line_count());
        self.exit_visual();
        self.cursor.set_row(start);
        self.apply_desired_column();
        self.normalize_view();
    }

    fn apply_command_preview(&self, rows: &[usize], lines: &mut [String]) {
        let Some(preview) = self.command_preview.as_ref() else {
            return;
        };
        for (row, text) in &preview.lines {
            if let Some(index) = rows.iter().position(|candidate| candidate == row) {
                if let Some(line) = lines.get_mut(index) {
                    *line = text.clone();
                }
            }
        }
    }

    pub(super) fn refresh_folds(&mut self) {
        let indent = self.effective_indent();
        let shiftwidth = if indent.shiftwidth == 0 {
            indent.tabstop
        } else {
            indent.shiftwidth
        };
        self.folds.set_indent(shiftwidth, indent.tabstop);
        self.folds.refresh(&self.buffer);
    }

    pub(super) fn reveal_hidden_cursor(&mut self) {
        if self.folds.is_hidden(self.cursor.row()) {
            self.reveal_folds_at(self.cursor.row());
        }
    }

    pub(super) fn normalize_view(&mut self) {
        let line_count = self.buffer.line_count();
        let top = self.folds.visible_top(self.viewport.top(), line_count);
        self.viewport.place_top(top, line_count);
    }
}

#[cfg(test)]
mod tests;
