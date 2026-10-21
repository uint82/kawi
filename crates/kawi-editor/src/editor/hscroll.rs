use super::Editor;

impl Editor {
    pub fn view_leftcol(&self) -> usize {
        self.viewport.leftcol()
    }

    pub fn view_tabstop(&self) -> usize {
        self.effective_indent().tabstop
    }

    pub fn reveal_cursor_horizontally(&mut self) {
        let width = self.text_width();
        if width == 0 {
            return;
        }
        let (start, virt, end) = self.cursor_horizontal_span();
        let leftcol = self.viewport.leftcol();
        let off_left = start as isize - leftcol as isize;
        let off_right = end as isize - leftcol as isize - width as isize + 1;
        if off_left >= 0 && off_right <= 0 {
            return;
        }
        let diff = (-off_left).max(off_right);
        let leftcol = if diff as usize >= width / 2 || off_right >= off_left {
            virt.saturating_sub(width / 2)
        } else if off_left < 0 {
            leftcol.saturating_sub(diff as usize)
        } else {
            leftcol.saturating_add(diff as usize)
        };
        self.viewport.set_leftcol(leftcol);
    }

    pub(super) fn scroll_left(&mut self) {
        let count = self.take_count();
        self.set_view_leftcol(self.viewport.leftcol().saturating_sub(count));
    }

    pub(super) fn scroll_right(&mut self) {
        let count = self.take_count();
        self.set_view_leftcol(self.viewport.leftcol().saturating_add(count));
    }

    pub(super) fn scroll_half_left(&mut self) {
        let columns = self.half_scroll_columns();
        self.set_view_leftcol(self.viewport.leftcol().saturating_sub(columns));
    }

    pub(super) fn scroll_half_right(&mut self) {
        let columns = self.half_scroll_columns();
        self.set_view_leftcol(self.viewport.leftcol().saturating_add(columns));
    }

    fn half_scroll_columns(&mut self) -> usize {
        self.take_count().saturating_mul(self.viewport.width() / 2)
    }

    pub fn scroll_wheel_horizontal(&mut self, delta: isize, page: bool) {
        if delta == 0 {
            return;
        }
        let columns = if page {
            self.viewport.width()
        } else {
            delta.unsigned_abs()
        };
        let leftcol = if delta < 0 {
            self.viewport.leftcol().saturating_sub(columns)
        } else {
            self.viewport.leftcol().saturating_add(columns)
        };
        self.move_cursor_to_longest_visible_line(leftcol);
        self.set_view_leftcol(leftcol);
    }

    fn move_cursor_to_longest_visible_line(&mut self, leftcol: usize) {
        let cursor_row = self.cursor.row();
        if leftcol <= self.scroll_line_len(cursor_row) {
            return;
        }
        let Some(row) = self.longest_visible_line() else {
            return;
        };
        self.cursor.set_position(row, 0);
    }

    fn scroll_line_len(&self, row: usize) -> usize {
        let len = self.buffer.line_len(row);
        if len == 0 {
            return 0;
        }
        let tabstop = self.effective_indent().tabstop;
        self.buffer.screen_col(row, len - 1, tabstop)
    }

    fn longest_visible_line(&self) -> Option<usize> {
        let cursor_row = self.cursor.row();
        let top = self.viewport.top();
        let end = self.viewport.visible_end().min(self.buffer.line_count());
        if cursor_row < top || cursor_row >= end {
            return None;
        }
        let mut best = None;
        let mut width = 0;
        for row in top..end {
            let len = self.scroll_line_len(row);
            if len > width {
                width = len;
                best = Some(row);
            } else if len == width {
                if let Some(current) = best {
                    if row.abs_diff(cursor_row) < current.abs_diff(cursor_row) {
                        best = Some(row);
                    }
                }
            }
        }
        best
    }

    pub(super) fn scroll_cursor_start(&mut self) {
        let (start, _, _) = self.cursor_horizontal_span();
        self.viewport.set_leftcol(start);
    }

    pub(super) fn scroll_cursor_end(&mut self) {
        let width = self.text_width();
        if width == 0 {
            return;
        }
        let (_, _, end) = self.cursor_horizontal_span();
        let leftcol = if end < width { 0 } else { end - width + 1 };
        self.viewport.set_leftcol(leftcol);
    }

    fn set_view_leftcol(&mut self, leftcol: usize) {
        if self.viewport.leftcol() == leftcol {
            return;
        }
        self.viewport.set_leftcol(leftcol);
        let width = self.text_width();
        if width == 0 {
            return;
        }
        let lastcol = leftcol.saturating_add(width).saturating_sub(1);
        let (_, virt, _) = self.cursor_horizontal_span();
        let mut moved = false;
        if virt > lastcol {
            moved = self.set_cursor_screen(lastcol);
        } else if virt < leftcol {
            moved = self.set_cursor_screen(leftcol);
        }
        let (start, _, end) = self.cursor_horizontal_span();
        if end > lastcol {
            moved |= self.set_cursor_screen(start.saturating_sub(1));
        } else if start < leftcol {
            if self.advance_cursor_screen(end.saturating_add(1)) {
                moved = true;
            } else {
                self.viewport.set_leftcol(start);
                moved = true;
            }
        }
        if moved {
            self.remember_wanted_column();
        }
    }

    fn set_cursor_screen(&mut self, screen: usize) -> bool {
        let row = self.cursor.row();
        let tabstop = self.effective_indent().tabstop;
        let col = self.buffer.char_col(row, screen, tabstop);
        let moved = col != self.cursor.col();
        self.cursor.set_col(col);
        moved
    }

    fn advance_cursor_screen(&mut self, screen: usize) -> bool {
        let row = self.cursor.row();
        let tabstop = self.effective_indent().tabstop;
        let len = self.buffer.line_len(row);
        if len == 0 || screen >= self.buffer.screen_col(row, len, tabstop) {
            return false;
        }
        self.set_cursor_screen(screen)
    }

    fn text_width(&self) -> usize {
        self.viewport.width().saturating_sub(self.gutter_width())
    }

    fn cursor_horizontal_span(&self) -> (usize, usize, usize) {
        let row = self.cursor.row();
        let col = self.cursor.col();
        let tabstop = self.effective_indent().tabstop;
        let len = self.buffer.line_len(row);
        let start = self.buffer.screen_col(row, col.min(len), tabstop);
        let end = if col < len {
            self.buffer
                .screen_col(row, col.saturating_add(1), tabstop)
                .saturating_sub(1)
        } else {
            start
        };
        (start, self.cursor_screen_col(), end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    const LONG: &str = "012345678901234567890123456789012345678901234567890123456789\nshort\n";

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn view(text: &str) -> Editor {
        let mut editor = editor_with(text);
        editor.set_view_size(20, 5);
        editor
    }

    fn left(editor: &Editor) -> usize {
        editor.view_leftcol()
    }

    fn put(editor: &mut Editor, row: usize, col: usize) {
        editor.apply(Action::JumpTo(row, col));
    }

    #[test]
    fn cursor_inside_view_keeps_leftcol() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 15);
        assert_eq!(left(&editor), 0);
    }

    #[test]
    fn cursor_one_past_right_edge_scrolls_one() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 20);
        assert_eq!(left(&editor), 5);
    }

    #[test]
    fn cursor_near_right_edge_scrolls_minimum() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 24);
        assert_eq!(left(&editor), 16);
    }

    #[test]
    fn far_jump_centers_cursor() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 40);
        assert_eq!(left(&editor), 32);
    }

    #[test]
    fn line_end_centers_last_char() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 59);
        assert_eq!(left(&editor), 51);
    }

    #[test]
    fn moving_to_short_line_resets_leftcol() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 40);
        put(&mut editor, 1, 4);
        assert_eq!(left(&editor), 0);
    }

    #[test]
    fn vertical_return_restores_centered_view() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 40);
        put(&mut editor, 1, 4);
        put(&mut editor, 0, 40);
        assert_eq!(left(&editor), 32);
    }

    #[test]
    fn number_gutter_uses_text_width() {
        let mut editor = view(LONG);
        editor.set_number(true);
        put(&mut editor, 0, 20);
        assert_eq!(left(&editor), 5);
    }

    #[test]
    fn number_gutter_centers_with_text_width() {
        let mut editor = view(LONG);
        editor.set_number(true);
        put(&mut editor, 0, 40);
        assert_eq!(left(&editor), 32);
    }

    #[test]
    fn scroll_right_moves_cursor_to_visible_start() {
        let mut editor = view(LONG);
        editor.apply(Action::ScrollRight);
        assert_eq!(left(&editor), 1);
        assert_eq!(editor.col(), 1);
    }

    #[test]
    fn scroll_right_count_moves_cursor() {
        let mut editor = view(LONG);
        editor.apply(Action::PushCount(3));
        editor.apply(Action::ScrollRight);
        assert_eq!(left(&editor), 3);
        assert_eq!(editor.col(), 3);
    }

    #[test]
    fn scroll_right_stops_at_last_char() {
        let mut editor = view("abc\n");
        for _ in 0..5 {
            editor.apply(Action::ScrollRight);
        }
        assert_eq!(left(&editor), 2);
        assert_eq!(editor.col(), 2);
    }

    #[test]
    fn scroll_left_keeps_cursor_in_place() {
        let mut editor = view(LONG);
        for _ in 0..2 {
            editor.apply(Action::ScrollRight);
        }
        assert_eq!(left(&editor), 2);
        assert_eq!(editor.col(), 2);
        editor.apply(Action::ScrollLeft);
        assert_eq!(left(&editor), 1);
        assert_eq!(editor.col(), 2);
    }

    #[test]
    fn scroll_half_right_pulls_cursor_into_view() {
        let mut editor = view(LONG);
        editor.apply(Action::ScrollHalfRight);
        assert_eq!(left(&editor), 10);
        assert_eq!(editor.col(), 10);
    }

    #[test]
    fn scroll_half_uses_window_width_with_number() {
        let mut editor = view(LONG);
        editor.set_number(true);
        editor.apply(Action::ScrollHalfRight);
        assert_eq!(left(&editor), 10);
    }

    #[test]
    fn scroll_half_left_reveals_earlier_text() {
        let mut editor = view(LONG);
        editor.apply(Action::ScrollHalfRight);
        editor.apply(Action::ScrollHalfLeft);
        assert_eq!(left(&editor), 0);
    }

    #[test]
    fn cursor_start_places_cursor_at_left_edge() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 30);
        editor.apply(Action::ScrollCursorStart);
        assert_eq!(left(&editor), 30);
        assert_eq!(editor.col(), 30);
    }

    #[test]
    fn cursor_end_places_cursor_at_right_edge() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 30);
        editor.apply(Action::ScrollCursorEnd);
        assert_eq!(left(&editor), 15);
        assert_eq!(editor.col(), 30);
    }

    #[test]
    fn cursor_end_uses_last_cell_of_tab() {
        let mut editor = view("\tabc\n");
        editor.apply(Action::ScrollCursorEnd);
        assert_eq!(left(&editor), 0);
    }

    #[test]
    fn scroll_right_leaves_partially_cut_tab() {
        let mut editor = view("\tabcdefghijklmnopqrstuvwxyz\n");
        editor.apply(Action::ScrollRight);
        assert_eq!(left(&editor), 1);
        assert_eq!(editor.col(), 1);
    }

    #[test]
    fn wheel_right_scrolls_six_columns() {
        let mut editor = view(LONG);
        editor.scroll_wheel_horizontal(6, false);
        assert_eq!(left(&editor), 6);
        assert_eq!(editor.col(), 6);
    }

    #[test]
    fn wheel_left_at_left_edge_stays() {
        let mut editor = view(LONG);
        editor.scroll_wheel_horizontal(-6, false);
        assert_eq!(left(&editor), 0);
        assert_eq!(editor.col(), 0);
    }

    #[test]
    fn wheel_page_uses_window_width() {
        let mut editor = view(LONG);
        editor.set_number(true);
        editor.scroll_wheel_horizontal(6, true);
        assert_eq!(left(&editor), 20);
        assert_eq!(editor.col(), 20);
    }

    #[test]
    fn wheel_jumps_to_longest_visible_line() {
        let mut editor =
            view("abc\n012345678901234567890123456789012345678901234567890123456789\nxyz\n");
        editor.scroll_wheel_horizontal(6, false);
        assert_eq!(editor.row(), 1);
        assert_eq!(left(&editor), 6);
        assert_eq!(editor.col(), 6);
    }

    #[test]
    fn wheel_treats_tab_at_line_end_as_short() {
        let mut editor = view("\t\n012345678901234567890123456789012345678901234567890123456789\n");
        editor.scroll_wheel_horizontal(6, false);
        assert_eq!(editor.row(), 1);
        assert_eq!(left(&editor), 6);
    }

    #[test]
    fn insert_past_right_edge_scrolls() {
        let mut editor = view(LONG);
        put(&mut editor, 0, 15);
        editor.apply(Action::Append);
        for _ in 0..10 {
            editor.apply(Action::InsertChar('x'));
        }
        let col = editor.cursor_screen_col();
        assert!(editor.view_leftcol() > 0);
        assert!(col >= editor.view_leftcol());
        assert!(col < editor.view_leftcol() + 20);
    }
}
