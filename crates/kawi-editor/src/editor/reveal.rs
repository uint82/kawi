use super::Editor;

impl Editor {
    pub fn reveal_cursor(&mut self) {
        self.reveal_hidden_cursor();
        let row = self.cursor.row();
        let row_display = self.display_row(row);
        let top_display = self.display_row(self.visible_top());
        let height = self.view_height();
        if row_display >= top_display && row_display < top_display.saturating_add(height) {
            return;
        }
        let top_display = if row_display < top_display {
            self.reveal_above_display(row_display, top_display)
        } else {
            self.reveal_below_display(row_display, top_display, height)
        };
        let top_display = top_display.min(self.display_row(self.max_view_top()));
        let top = self.buffer_row_at_display(top_display);
        self.viewport.place_top(top, self.buffer.line_count());
    }

    fn reveal_above_display(&self, row: usize, top: usize) -> usize {
        let under = top.saturating_sub(row);
        if under < self.half_height() {
            row
        } else {
            row.saturating_sub(self.center_offset())
        }
    }

    fn reveal_below_display(&self, row: usize, top: usize, height: usize) -> usize {
        let over = row.saturating_sub(top.saturating_add(height));
        if over <= self.half_height() {
            row.saturating_sub(height.saturating_sub(1))
        } else if over <= height {
            row.saturating_sub(height / 2)
        } else {
            row.saturating_sub(self.center_offset())
        }
    }

    fn half_height(&self) -> usize {
        (self.view_height() / 2).saturating_sub(1).max(2)
    }

    fn center_offset(&self) -> usize {
        self.view_height().saturating_sub(1) / 2
    }
}
