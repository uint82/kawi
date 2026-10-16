use super::Editor;

impl Editor {
    pub fn view_top(&self) -> usize {
        self.viewport.top()
    }

    pub fn view_height(&self) -> usize {
        self.viewport.height()
    }

    pub fn viewport(&self) -> &crate::viewport::Viewport {
        &self.viewport
    }

    pub fn set_view_size(&mut self, width: usize, height: usize) {
        self.viewport.set_size(width, height);
    }

    pub fn reveal_cursor(&mut self) {
        self.viewport.ensure_visible(self.cursor.row());
    }

    pub fn scroll_wheel(&mut self, delta: isize) {
        self.viewport.scroll_lines(delta, self.buffer.line_count());
        let row = self.cursor.row().clamp(
            self.viewport.top(),
            self.viewport.visible_end().saturating_sub(1),
        );
        if row != self.cursor.row() {
            self.cursor.set_position(row, self.cursor.col());
        }
    }

    pub(super) fn scroll_half_up(&mut self) {
        self.track_insert_move();
        let amount = self.scroll_count_or(self.half_page());
        let top = self
            .viewport
            .top()
            .saturating_sub(amount)
            .min(self.max_top());
        self.viewport.place_top(top, self.buffer.line_count());
        self.keep_column_in_view(self.cursor.row().saturating_sub(amount));
    }

    pub(super) fn scroll_half_down(&mut self) {
        self.track_insert_move();
        let amount = self.scroll_count_or(self.half_page());
        let top = self
            .viewport
            .top()
            .saturating_add(amount)
            .min(self.max_top());
        self.viewport.place_top(top, self.buffer.line_count());
        self.keep_column_in_view(self.cursor.row().saturating_add(amount));
    }

    pub(super) fn scroll_page_up(&mut self) {
        self.track_insert_move();
        let pages = self.take_count();
        let top = self
            .viewport
            .top()
            .saturating_sub(pages.saturating_mul(self.page_lines()));
        self.scroll_page_to(top, true);
    }

    pub(super) fn scroll_page_down(&mut self) {
        self.track_insert_move();
        let pages = self.take_count();
        let candidate = self
            .viewport
            .top()
            .saturating_add(pages.saturating_mul(self.page_lines()));
        let last = self.buffer.line_count().saturating_sub(1);
        let top = if candidate >= last.saturating_sub(1) {
            last
        } else {
            candidate
        };
        self.scroll_page_to(top, false);
    }

    pub(super) fn move_to_viewport_top(&mut self) {
        self.track_insert_move();
        let target = self
            .viewport
            .top()
            .saturating_add(self.take_count().saturating_sub(1))
            .min(self.last_row());
        self.keep_column_to(target);
    }

    pub(super) fn move_to_viewport_middle(&mut self) {
        self.track_insert_move();
        let target = self.viewport.top().saturating_add(self.middle_offset());
        self.keep_column_to(target.min(self.last_row()));
    }

    pub(super) fn move_to_viewport_bottom(&mut self) {
        self.track_insert_move();
        let target = self
            .visible_bot()
            .saturating_sub(self.take_count().saturating_sub(1))
            .max(self.viewport.top());
        self.keep_column_to(target);
    }

    pub(super) fn place_center(&mut self) {
        self.jump_to_count_line();
        let offset = self.view_height().saturating_sub(1) / 2;
        let top = self.cursor.row().saturating_sub(offset);
        self.viewport.place_top(top, self.buffer.line_count());
    }

    pub(super) fn place_top(&mut self) {
        self.jump_to_count_line();
        self.viewport
            .place_top(self.cursor.row(), self.buffer.line_count());
    }

    pub(super) fn place_bottom(&mut self) {
        self.jump_to_count_line();
        let top = self
            .cursor
            .row()
            .saturating_sub(self.view_height().saturating_sub(1));
        self.viewport.place_top(top, self.buffer.line_count());
    }

    fn scroll_page_to(&mut self, top: usize, backward: bool) {
        let moved = top != self.viewport.top();
        self.viewport.place_top(top, self.buffer.line_count());
        if !moved {
            return;
        }
        if backward {
            self.keep_column_to(self.visible_bot());
        } else {
            self.keep_column_to(self.viewport.top());
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
        let top = self.viewport.top();
        self.keep_column_to(row.clamp(top, self.visible_bot()));
    }

    fn keep_column_to(&mut self, row: usize) {
        self.cursor.set_position(row, self.cursor.col());
    }

    fn scroll_count_or(&mut self, fallback: usize) -> usize {
        // TODO(scroll): persist counted amount across bare repeats like vim 'scroll'.
        if self.has_count() {
            self.take_count()
        } else {
            fallback
        }
    }

    fn half_page(&self) -> usize {
        (self.view_height() / 2).max(1)
    }

    fn page_lines(&self) -> usize {
        self.view_height().saturating_sub(2).max(1)
    }

    fn max_top(&self) -> usize {
        self.buffer.line_count().saturating_sub(self.view_height())
    }

    fn visible_bot(&self) -> usize {
        self.viewport
            .top()
            .saturating_add(self.view_height().saturating_sub(1))
            .min(self.last_row())
    }

    fn middle_offset(&self) -> usize {
        let visible = self
            .view_height()
            .min(self.buffer.line_count().saturating_sub(self.viewport.top()));
        let half = visible.saturating_add(1) / 2;
        half.saturating_sub(1).min(visible.saturating_sub(1))
    }

    fn last_row(&self) -> usize {
        self.buffer.line_count().saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::mode::Mode;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn tall_editor() -> Editor {
        let text = (1..=100)
            .map(|i| format!("    {i} abcdef"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.set_view_size(80, 20);
        e
    }

    fn place_top(e: &mut Editor, top: usize) {
        e.viewport.place_top(top, e.buffer.line_count());
    }

    fn push_digits(e: &mut Editor, digits: &[u8]) {
        for digit in digits {
            e.apply(Action::PushCount(*digit));
        }
    }

    #[test]
    fn half_down_moves_view_and_cursor() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 7));
        e.apply(Action::ScrollHalfDown);
        assert_eq!((e.row(), e.col()), (59, 7));
        assert_eq!(e.view_top(), 50);
    }

    #[test]
    fn half_up_moves_view_and_cursor() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 7));
        e.apply(Action::ScrollHalfUp);
        assert_eq!((e.row(), e.col()), (39, 7));
        assert_eq!(e.view_top(), 30);
    }

    #[test]
    fn half_down_pins_cursor_to_bottom() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(59, 0));
        e.apply(Action::ScrollHalfDown);
        assert_eq!((e.row(), e.col()), (69, 0));
        assert_eq!(e.view_top(), 50);
    }

    #[test]
    fn half_up_pins_cursor_to_top() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(40, 0));
        e.apply(Action::ScrollHalfUp);
        assert_eq!((e.row(), e.col()), (30, 0));
        assert_eq!(e.view_top(), 30);
    }

    #[test]
    fn half_up_at_top_clamps_cursor() {
        let mut e = tall_editor();
        e.apply(Action::JumpTo(5, 6));
        e.apply(Action::ScrollHalfUp);
        assert_eq!((e.row(), e.col()), (0, 6));
        assert_eq!(e.view_top(), 0);
    }

    #[test]
    fn half_down_at_bottom_clamps_cursor() {
        let mut e = tall_editor();
        place_top(&mut e, 80);
        e.apply(Action::JumpTo(94, 6));
        e.apply(Action::ScrollHalfDown);
        assert_eq!((e.row(), e.col()), (99, 6));
        assert_eq!(e.view_top(), 80);
    }

    #[test]
    fn half_down_uses_count() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[5]);
        e.apply(Action::ScrollHalfDown);
        assert_eq!((e.row(), e.col()), (54, 0));
        assert_eq!(e.view_top(), 45);
    }

    #[test]
    fn page_down_moves_cursor_to_first_line() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        e.apply(Action::ScrollPageDown);
        assert_eq!((e.row(), e.col()), (58, 0));
        assert_eq!(e.view_top(), 58);
    }

    #[test]
    fn page_up_moves_cursor_to_last_line() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        e.apply(Action::ScrollPageUp);
        assert_eq!((e.row(), e.col()), (41, 0));
        assert_eq!(e.view_top(), 22);
    }

    #[test]
    fn page_down_uses_count() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[2]);
        e.apply(Action::ScrollPageDown);
        assert_eq!((e.row(), e.col()), (76, 0));
        assert_eq!(e.view_top(), 76);
    }

    #[test]
    fn page_down_near_end_snaps_to_last_line() {
        let mut e = tall_editor();
        place_top(&mut e, 80);
        e.apply(Action::JumpTo(80, 0));
        e.apply(Action::ScrollPageDown);
        assert_eq!((e.row(), e.col()), (99, 0));
        assert_eq!(e.view_top(), 99);
    }

    #[test]
    fn page_up_near_start_lands_on_first_line() {
        let mut e = tall_editor();
        place_top(&mut e, 5);
        e.apply(Action::JumpTo(14, 0));
        e.apply(Action::ScrollPageUp);
        assert_eq!((e.row(), e.col()), (19, 0));
        assert_eq!(e.view_top(), 0);
    }

    #[test]
    fn page_scroll_without_room_keeps_cursor() {
        let mut e = tall_editor();
        e.apply(Action::JumpTo(4, 0));
        e.apply(Action::ScrollPageUp);
        assert_eq!((e.row(), e.col()), (4, 0));
        assert_eq!(e.view_top(), 0);
    }

    #[test]
    fn viewport_top_middle_bottom_keep_column() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 7));
        e.apply(Action::MoveToViewportTop);
        assert_eq!((e.row(), e.col()), (40, 7));
        e.apply(Action::JumpTo(49, 7));
        e.apply(Action::MoveToViewportMiddle);
        assert_eq!((e.row(), e.col()), (49, 7));
        e.apply(Action::JumpTo(49, 7));
        e.apply(Action::MoveToViewportBottom);
        assert_eq!((e.row(), e.col()), (59, 7));
        assert_eq!(e.view_top(), 40);
    }

    #[test]
    fn viewport_top_uses_count() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[2]);
        e.apply(Action::MoveToViewportTop);
        assert_eq!((e.row(), e.col()), (41, 0));
    }

    #[test]
    fn viewport_bottom_clamps_to_top() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[2, 1]);
        e.apply(Action::MoveToViewportBottom);
        assert_eq!((e.row(), e.col()), (40, 0));
    }

    #[test]
    fn viewport_middle_ignores_count() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[3]);
        e.apply(Action::MoveToViewportMiddle);
        assert_eq!((e.row(), e.col()), (49, 0));
    }

    #[test]
    fn viewport_middle_of_short_file() {
        let mut e = editor_with("    1\n    2\n    3");
        e.set_view_size(80, 20);
        e.apply(Action::JumpTo(0, 0));
        e.apply(Action::MoveToViewportMiddle);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn place_top_bottom_center() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        e.apply(Action::PlaceTop);
        assert_eq!(e.view_top(), 49);
        assert_eq!((e.row(), e.col()), (49, 0));
        e.apply(Action::PlaceBottom);
        assert_eq!(e.view_top(), 30);
        assert_eq!((e.row(), e.col()), (49, 0));
        e.apply(Action::PlaceCenter);
        assert_eq!(e.view_top(), 40);
        assert_eq!((e.row(), e.col()), (49, 0));
    }

    #[test]
    fn place_center_near_end() {
        let mut e = tall_editor();
        e.apply(Action::JumpTo(94, 6));
        e.apply(Action::PlaceCenter);
        assert_eq!(e.view_top(), 85);
        assert_eq!((e.row(), e.col()), (94, 6));
    }

    #[test]
    fn place_top_near_end_sticks() {
        let mut e = tall_editor();
        e.apply(Action::JumpTo(94, 6));
        e.apply(Action::PlaceTop);
        assert_eq!(e.view_top(), 94);
        assert_eq!((e.row(), e.col()), (94, 6));
    }

    #[test]
    fn place_bottom_near_end() {
        let mut e = tall_editor();
        e.apply(Action::JumpTo(94, 6));
        e.apply(Action::PlaceBottom);
        assert_eq!(e.view_top(), 75);
        assert_eq!((e.row(), e.col()), (94, 6));
    }

    #[test]
    fn place_center_uses_count() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        push_digits(&mut e, &[3, 0]);
        e.apply(Action::PlaceCenter);
        assert_eq!((e.row(), e.col()), (29, 0));
        assert_eq!(e.view_top(), 20);
    }

    #[test]
    fn visual_scroll_extends_head() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveToViewportTop);
        assert_eq!(e.selection_range(), Some(((40, 0), (49, 0))));
        assert_eq!((e.row(), e.col()), (40, 0));
        assert_eq!(e.mode(), Mode::Visual);
    }

    #[test]
    fn visual_half_scroll_extends_head() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(49, 0));
        e.apply(Action::EnterVisual);
        e.apply(Action::ScrollHalfDown);
        assert_eq!(e.selection_range(), Some(((49, 0), (59, 0))));
        assert_eq!((e.row(), e.col()), (59, 0));
        assert_eq!(e.view_top(), 50);
    }

    #[test]
    fn wheel_scroll_pins_cursor() {
        let mut e = tall_editor();
        place_top(&mut e, 40);
        e.apply(Action::JumpTo(59, 3));
        e.scroll_wheel(3);
        assert_eq!(e.view_top(), 43);
        assert_eq!((e.row(), e.col()), (59, 3));
        e.scroll_wheel(30);
        assert_eq!(e.view_top(), 73);
        assert_eq!((e.row(), e.col()), (73, 3));
    }

    #[test]
    fn reveal_cursor_scrolls_into_view() {
        let mut e = tall_editor();
        e.apply(Action::JumpTo(50, 0));
        e.reveal_cursor();
        assert_eq!(e.view_top(), 31);
    }
}
