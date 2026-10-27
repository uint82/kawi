use super::{Editor, VisibleWindow};
use crate::folds::FoldState;

impl Editor {
    pub fn buffer_visible_top(&self, number: usize, top: usize) -> Option<usize> {
        let buffer = self.buffer_ref(number)?;
        let folds = self.fold_state_for(number)?;
        Some(folds.visible_top(top, buffer.line_count()))
    }

    pub fn buffer_visible_rows(
        &self,
        number: usize,
        top: usize,
        height: usize,
    ) -> Option<Vec<usize>> {
        let buffer = self.buffer_ref(number)?;
        let folds = self.fold_state_for(number)?;
        Some(folds.visible_rows(top, height, buffer.line_count()))
    }

    pub fn buffer_visible_window(
        &self,
        number: usize,
        top: usize,
        height: usize,
    ) -> Option<VisibleWindow> {
        let buffer = self.buffer_ref(number)?;
        let rows = self.buffer_visible_rows(number, top, height)?;
        let lines = rows
            .iter()
            .map(|row| buffer.line(*row).unwrap_or_default())
            .collect();
        Some(VisibleWindow { rows, lines })
    }

    pub fn buffer_visible_line_count(&self, number: usize) -> Option<usize> {
        let buffer = self.buffer_ref(number)?;
        let folds = self.fold_state_for(number)?;
        Some(folds.visible_line_count(buffer.line_count()))
    }

    pub fn buffer_display_row(&self, number: usize, row: usize) -> Option<usize> {
        let buffer = self.buffer_ref(number)?;
        let folds = self.fold_state_for(number)?;
        Some(folds.display_row(row, buffer.line_count()))
    }

    pub fn buffer_fold_marker(&self, number: usize, row: usize) -> Option<char> {
        let folds = self.fold_state_for(number)?;
        Some(folds.marker(row))
    }

    pub(crate) fn buffer_ref(&self, number: usize) -> Option<&kawi_buffer::Buffer> {
        if number == self.buffers.current() {
            Some(&self.buffer)
        } else {
            self.buffers.buffer_ref(number)
        }
    }

    pub(crate) fn fold_state_for(&self, number: usize) -> Option<&FoldState> {
        if number == self.buffers.current() {
            Some(&self.folds)
        } else {
            self.buffers.fold_state(number)
        }
    }
}
