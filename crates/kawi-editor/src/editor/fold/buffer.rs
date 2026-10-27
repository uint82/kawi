use super::{Editor, VisibleWindow};
use crate::folds::{FoldSnapshot, FoldState};

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

    pub fn buffer_fold_snapshot(&self, number: usize) -> Option<FoldSnapshot> {
        self.fold_state_for(number).map(FoldState::snapshot)
    }

    pub fn buffer_content_hash(&self, number: usize) -> Option<u64> {
        self.buffer_ref(number)
            .map(kawi_buffer::Buffer::content_hash)
    }

    pub fn restore_buffer_folds(&mut self, number: usize, snapshot: FoldSnapshot) -> bool {
        let Some(line_count) = self.buffer_ref(number).map(kawi_buffer::Buffer::line_count) else {
            return false;
        };
        if number == self.buffers.current() {
            self.folds.restore(snapshot, line_count);
            self.normalize_view();
            return true;
        }
        let Some(folds) = self.buffers.parked_fold_state_mut(number) else {
            return false;
        };
        folds.restore(snapshot, line_count);
        true
    }

    pub fn refresh_buffer_fold_state(&mut self, number: usize) -> bool {
        if !self.refresh_buffer_folds(number) {
            return false;
        }
        self.revalidate_buffer_folds(number)
    }

    pub(crate) fn revalidate_folds_after_change(
        &mut self,
        number: usize,
        previous_hash: Option<u64>,
    ) -> bool {
        let Some(current_hash) = self.buffer_content_hash(number) else {
            return false;
        };
        if previous_hash != Some(current_hash) {
            self.drop_buffer_manual_folds(number);
        }
        self.refresh_buffer_folds(number);
        self.revalidate_buffer_folds(number);
        true
    }

    fn drop_buffer_manual_folds(&mut self, number: usize) -> bool {
        if number == self.buffers.current() {
            self.folds.drop_manual();
            return true;
        }
        let Some(folds) = self.buffers.parked_fold_state_mut(number) else {
            return false;
        };
        folds.drop_manual();
        true
    }

    fn refresh_buffer_folds(&mut self, number: usize) -> bool {
        if number == self.buffers.current() {
            self.refresh_folds();
            return true;
        }
        self.buffers.refresh_parked_folds(number)
    }

    fn revalidate_buffer_folds(&mut self, number: usize) -> bool {
        let Some(line_count) = self.buffer_ref(number).map(kawi_buffer::Buffer::line_count) else {
            return false;
        };
        if number == self.buffers.current() {
            self.folds.revalidate(line_count);
            self.normalize_view();
            return true;
        }
        let Some(folds) = self.buffers.parked_fold_state_mut(number) else {
            return false;
        };
        folds.revalidate(line_count);
        true
    }
}
