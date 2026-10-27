use super::Editor;

impl Editor {
    pub fn view_top(&self) -> usize {
        self.visible_top()
    }

    pub fn view_height(&self) -> usize {
        self.viewport.height()
    }

    pub fn viewport(&self) -> &crate::viewport::Viewport {
        &self.viewport
    }

    pub fn set_view_size(&mut self, width: usize, height: usize) {
        let previous_height = self.viewport.height();
        self.viewport.set_size(width, height);
        if self.viewport.height() != previous_height {
            self.scroll_amount = None;
        }
    }

    pub fn scroll_wheel(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        let top = self.visible_top();
        let top = if delta < 0 {
            self.visible_row_before(top, delta.unsigned_abs())
        } else {
            self.visible_row_after(top, delta.unsigned_abs())
        };
        let top = top.min(self.max_view_top());
        self.viewport.place_top(top, self.buffer.line_count());
        self.clamp_row_to_view();
    }

    pub(super) fn scroll_line_up(&mut self) {
        self.scroll_line_by(-1);
    }

    pub(super) fn scroll_line_down(&mut self) {
        self.scroll_line_by(1);
    }

    fn scroll_line_by(&mut self, direction: isize) {
        self.track_insert_move();
        let count = self.take_count() as isize;
        let delta = direction.saturating_mul(count);
        let top = self.visible_top();
        let top = if delta < 0 {
            self.visible_row_before(top, delta.unsigned_abs())
        } else {
            self.visible_row_after(top, delta.unsigned_abs())
        };
        let top = top.min(self.last_row());
        self.viewport.place_top(top, self.buffer.line_count());
        self.clamp_row_to_view();
    }

    fn clamp_row_to_view(&mut self) {
        let top = self.visible_top();
        let bottom = self.visible_bottom();
        let row = self.cursor.row();
        if row < top {
            self.keep_column_to(top);
        } else if row > bottom {
            self.keep_column_to(bottom);
        }
    }

    pub(super) fn scroll_half_up(&mut self) {
        self.scroll_half(false);
    }

    pub(super) fn scroll_half_down(&mut self) {
        self.scroll_half(true);
    }

    fn scroll_half(&mut self, forward: bool) {
        self.track_insert_move();
        let amount = self.half_scroll_amount();
        let top = if forward {
            self.visible_row_after(self.visible_top(), amount)
        } else {
            self.visible_row_before(self.visible_top(), amount)
        };
        self.viewport
            .place_top(top.min(self.max_view_top()), self.buffer.line_count());
        let row = if forward {
            self.visible_row_after(self.cursor.row(), amount)
        } else {
            self.visible_row_before(self.cursor.row(), amount)
        };
        self.keep_column_in_view(row);
    }

    pub(super) fn scroll_page_up(&mut self) {
        self.track_insert_move();
        let pages = self.take_count();
        let top =
            self.visible_row_before(self.visible_top(), pages.saturating_mul(self.page_lines()));
        self.scroll_page_to(top, true);
    }

    pub(super) fn scroll_page_down(&mut self) {
        self.track_insert_move();
        let pages = self.take_count();
        let candidate =
            self.visible_row_after(self.visible_top(), pages.saturating_mul(self.page_lines()));
        let last = self.last_visible_row();
        let top = if candidate >= last.saturating_sub(1) {
            last
        } else {
            candidate
        };
        self.scroll_page_to(top, false);
    }

    pub(super) fn move_to_viewport_top(&mut self) {
        self.track_insert_move();
        let count = self.take_count();
        let rows = self.viewport_rows();
        let target = rows
            .get(count.saturating_sub(1))
            .copied()
            .unwrap_or_else(|| self.last_visible_row());
        self.keep_column_to(target);
    }

    pub(super) fn move_to_viewport_middle(&mut self) {
        self.track_insert_move();
        let rows = self.viewport_rows();
        let target = rows
            .get(self.middle_offset())
            .copied()
            .unwrap_or_else(|| self.last_visible_row());
        self.keep_column_to(target);
    }

    pub(super) fn move_to_viewport_bottom(&mut self) {
        self.track_insert_move();
        let count = self.take_count();
        let rows = self.viewport_rows();
        let target = rows
            .get(rows.len().saturating_sub(count))
            .copied()
            .unwrap_or_else(|| self.last_visible_row());
        self.keep_column_to(target);
    }

    pub(super) fn viewport_top_target(&mut self) -> usize {
        let count = self.take_count();
        self.viewport_rows()
            .get(count.saturating_sub(1))
            .copied()
            .unwrap_or_else(|| self.last_visible_row())
    }

    pub(super) fn viewport_middle_target(&self) -> usize {
        self.viewport_rows()
            .get(self.middle_offset())
            .copied()
            .unwrap_or_else(|| self.last_visible_row())
    }

    pub(super) fn viewport_bottom_target(&mut self) -> usize {
        let count = self.take_count();
        let rows = self.viewport_rows();
        rows.get(rows.len().saturating_sub(count))
            .copied()
            .unwrap_or_else(|| self.last_visible_row())
    }

    pub(super) fn place_center(&mut self) {
        self.jump_to_count_line();
        let offset = self.view_height().saturating_sub(1) / 2;
        let top_display = self.display_row(self.cursor.row()).saturating_sub(offset);
        let top = self.buffer_row_at_display(top_display);
        self.viewport.place_top(top, self.buffer.line_count());
    }

    pub(super) fn place_top(&mut self) {
        self.jump_to_count_line();
        self.viewport
            .place_top(self.cursor.row(), self.buffer.line_count());
    }

    pub(super) fn place_bottom(&mut self) {
        self.jump_to_count_line();
        let top_display = self
            .display_row(self.cursor.row())
            .saturating_sub(self.view_height().saturating_sub(1));
        let top = self.buffer_row_at_display(top_display);
        self.viewport.place_top(top, self.buffer.line_count());
    }

    fn scroll_page_to(&mut self, top: usize, backward: bool) {
        let previous = self.visible_top();
        let moved = top != previous;
        self.viewport.place_top(top, self.buffer.line_count());
        if !moved {
            return;
        }
        if backward {
            self.keep_column_to(self.visible_bottom());
        } else {
            self.keep_column_to(self.visible_top());
        }
    }

    fn jump_to_count_line(&mut self) {
        if !self.has_count() {
            return;
        }
        let row = self.take_count().saturating_sub(1).min(self.last_row());
        self.keep_column_to(row);
    }

    fn keep_column_in_view(&mut self, row: usize) {
        let top = self.visible_top();
        self.keep_column_to(row.clamp(top, self.visible_bottom()));
    }

    fn keep_column_to(&mut self, row: usize) {
        self.cursor.set_row(row);
        self.reveal_hidden_cursor();
        self.apply_desired_column();
    }

    fn half_scroll_amount(&mut self) -> usize {
        if self.has_count() {
            self.persist_half_scroll();
        }
        self.scroll_amount.unwrap_or_else(|| self.half_page())
    }

    fn persist_half_scroll(&mut self) {
        self.scroll_amount = Some(self.take_count().min(self.view_height()));
    }

    fn half_page(&self) -> usize {
        (self.view_height() / 2).max(1)
    }

    fn page_lines(&self) -> usize {
        self.view_height().saturating_sub(2).max(1)
    }

    fn viewport_rows(&self) -> Vec<usize> {
        self.visible_rows(self.visible_top(), self.view_height())
    }

    fn middle_offset(&self) -> usize {
        let visible = self.viewport_rows().len();
        let half = visible.saturating_add(1) / 2;
        half.saturating_sub(1).min(visible.saturating_sub(1))
    }

    pub(super) fn last_row(&self) -> usize {
        self.buffer.line_count().saturating_sub(1)
    }

    fn last_visible_row(&self) -> usize {
        self.folds
            .prev_visible(self.last_row(), self.buffer.line_count())
    }
}

#[cfg(test)]
mod tests;
