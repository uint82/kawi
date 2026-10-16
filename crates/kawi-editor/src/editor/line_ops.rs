use super::Editor;

impl Editor {
    pub(super) fn delete_touched_lines(&mut self) {
        let (first, last) = self.touched_rows();
        let (row, col) = self.delete_line_range(first, last, self.cursor.col());
        self.collapse_to(row, col);
    }

    pub(super) fn delete_line_range(
        &mut self,
        first: usize,
        last: usize,
        col: usize,
    ) -> (usize, usize) {
        self.remove_lines(first, last);
        let row = first.min(self.buffer.line_count().saturating_sub(1));
        let max = self.buffer.line_len(row).saturating_sub(1);
        let col = col.min(max);
        self.cursor.set_position(row, col);
        (row, col)
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

    pub(super) fn delete_to_last_line(&mut self) {
        let (first, last) = self.goto_span(true);
        self.delete_line_range(first, last, self.cursor.col());
    }

    pub(super) fn delete_to_first_line(&mut self) {
        let (first, last) = self.goto_span(false);
        self.delete_line_range(first, last, self.cursor.col());
    }

    pub(super) fn delete_to_viewport_top(&mut self) {
        let target = self.viewport_top_target().min(self.last_row());
        let (first, last) = self.viewport_span(target);
        self.delete_line_range(first, last, self.cursor.col());
    }

    pub(super) fn delete_to_viewport_middle(&mut self) {
        let target = self.viewport_middle_target().min(self.last_row());
        let (first, last) = self.viewport_span(target);
        self.delete_line_range(first, last, self.cursor.col());
    }

    pub(super) fn delete_to_viewport_bottom(&mut self) {
        let target = self.viewport_bottom_target().max(self.view_top());
        let (first, last) = self.viewport_span(target);
        self.delete_line_range(first, last, self.cursor.col());
    }

    pub(super) fn yank_to_viewport_top(&mut self) {
        let target = self.viewport_top_target().min(self.last_row());
        let (first, last) = self.viewport_span(target);
        self.yank_viewport_span(first, last);
    }

    pub(super) fn yank_to_viewport_middle(&mut self) {
        let target = self.viewport_middle_target().min(self.last_row());
        let (first, last) = self.viewport_span(target);
        self.yank_viewport_span(first, last);
    }

    pub(super) fn yank_to_viewport_bottom(&mut self) {
        let target = self.viewport_bottom_target().max(self.view_top());
        let (first, last) = self.viewport_span(target);
        self.yank_viewport_span(first, last);
    }

    pub(super) fn change_to_viewport_top(&mut self) {
        let target = self.viewport_top_target().min(self.last_row());
        let (first, last) = self.viewport_span(target);
        self.change_lines(first, last);
    }

    pub(super) fn change_to_viewport_middle(&mut self) {
        let target = self.viewport_middle_target().min(self.last_row());
        let (first, last) = self.viewport_span(target);
        self.change_lines(first, last);
    }

    pub(super) fn change_to_viewport_bottom(&mut self) {
        let target = self.viewport_bottom_target().max(self.view_top());
        let (first, last) = self.viewport_span(target);
        self.change_lines(first, last);
    }

    fn viewport_span(&self, target: usize) -> (usize, usize) {
        let row = self.cursor.row();
        (row.min(target), row.max(target))
    }

    fn yank_viewport_span(&mut self, first: usize, last: usize) {
        self.yank_line_range(first, last);
        self.cursor.set_position(first, self.cursor.col());
    }

    pub(super) fn yank_to_last_line(&mut self) {
        let (first, last) = self.goto_span(true);
        self.yank_viewport_span(first, last);
    }

    pub(super) fn yank_to_first_line(&mut self) {
        let (first, last) = self.goto_span(false);
        self.yank_viewport_span(first, last);
    }

    pub(super) fn change_to_last_line(&mut self) {
        let (first, last) = self.goto_span(true);
        self.change_lines(first, last);
    }

    pub(super) fn change_to_first_line(&mut self) {
        let (first, last) = self.goto_span(false);
        self.change_lines(first, last);
    }

    fn goto_span(&mut self, last: bool) -> (usize, usize) {
        let row = self.cursor.row();
        let target = self.goto_target(last);
        (row.min(target), row.max(target))
    }

    pub(super) fn touched_rows(&self) -> (usize, usize) {
        let ((first, _), (last, _)) = self.operator_range();
        (first, last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn push_digits(editor: &mut Editor, digits: &[u8]) {
        for digit in digits {
            editor.apply(Action::PushCount(*digit));
        }
    }

    fn begin_sequence(editor: &mut Editor) {
        editor.apply(Action::BeginSequence);
    }

    #[test]
    fn delete_to_last_line() {
        let mut editor = editor_with("abcdef\ngh\nijkl");
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::MoveRight);
        editor.apply(Action::DeleteToLastLine);
        assert_eq!(editor.text_lines(), vec![String::new()]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert_eq!(editor.register_text(), "abcdef\ngh\nijkl");
        assert_eq!(
            editor.register_kind(),
            crate::register::RegisterKind::LineWise
        );
    }

    #[test]
    fn delete_to_last_line_partial() {
        let mut editor = editor_with("a\nbb\ncc\ndd");
        editor.apply(Action::MoveDown);
        editor.apply(Action::DeleteToLastLine);
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.line_count(), 1);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn delete_motion_count_targets_line() {
        let mut editor = editor_with("a\nbb\ncc\ndd");
        push_digits(&mut editor, &[2]);
        editor.apply(Action::DeleteToLastLine);
        assert_eq!(
            editor.text_lines(),
            vec!["cc".to_string(), "dd".to_string()]
        );
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn delete_operator_count_targets_line() {
        let mut editor = editor_with("a\nbb\ncc\ndd");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        push_digits(&mut editor, &[2]);
        editor.apply(Action::DeleteToLastLine);
        assert_eq!(editor.text_lines(), vec!["a".to_string(), "dd".to_string()]);
        assert_eq!((editor.row(), editor.col()), (1, 0));
        assert_eq!(editor.register_text(), "bb\ncc");
    }

    #[test]
    fn delete_many_lines_pastes_back() {
        let text = (0..500)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = editor_with(&text);
        editor.apply(Action::DeleteToLastLine);
        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.register_text(), text);
        editor.apply(Action::PasteAfter);
        assert_eq!(editor.line_count(), 500);
        assert_eq!(editor.text_lines()[499], "line 499");
    }

    #[test]
    fn delete_to_last_line_from_first_deletes_all() {
        let mut editor = editor_with("a\nbb\ncc");
        begin_sequence(&mut editor);
        editor.apply(Action::DeleteToLastLine);
        assert_eq!(editor.text_lines(), vec![String::new()]);
        assert_eq!(editor.line_count(), 1);
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert_eq!(editor.selection_range(), None);
    }

    #[test]
    fn delete_count_one_to_last_line_targets_first_line() {
        let mut editor = editor_with("a\nbb\ncc\ndd");
        editor.apply(Action::MoveDown);
        push_digits(&mut editor, &[1]);
        editor.apply(Action::DeleteToLastLine);
        assert_eq!(editor.text_lines()[0], "cc");
        assert_eq!(editor.text_lines()[1], "dd");
        assert_eq!(editor.line_count(), 2);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn delete_to_first_line_from_first_keeps_rest() {
        let mut editor = editor_with("a\nbb\ncc");
        begin_sequence(&mut editor);
        editor.apply(Action::DeleteToFirstLine);
        assert_eq!(editor.text_lines()[0], "bb");
        assert_eq!(editor.text_lines()[1], "cc");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn delete_count_one_to_first_line() {
        let mut editor = editor_with("a\nbb\ncc\ndd");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        push_digits(&mut editor, &[1]);
        editor.apply(Action::DeleteToFirstLine);
        assert_eq!(editor.text_lines(), vec!["dd".to_string()]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    #[test]
    fn delete_to_first_line() {
        let mut editor = editor_with("a\nbb\ncc");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        editor.apply(Action::DeleteToFirstLine);
        assert_eq!(editor.text_lines(), vec![String::new()]);
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert_eq!(editor.register_text(), "a\nbb\ncc");
    }

    #[test]
    fn yank_to_last_line_keeps_cursor() {
        let mut editor = editor_with("a\nbb\ncc");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveRight);
        editor.apply(Action::YankToLastLine);
        assert_eq!(
            editor.text_lines(),
            vec!["a".to_string(), "bb".to_string(), "cc".to_string()]
        );
        assert_eq!(editor.register_text(), "bb\ncc");
        assert_eq!(
            editor.register_kind(),
            crate::register::RegisterKind::LineWise
        );
        assert_eq!((editor.row(), editor.col()), (1, 1));
    }

    #[test]
    fn yank_to_first_line_moves_to_first() {
        let mut editor = editor_with("a\nbb\ncc");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        editor.apply(Action::YankToFirstLine);
        assert_eq!(editor.register_text(), "a\nbb\ncc");
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }

    fn tall_editor() -> Editor {
        let text = (1..=100)
            .map(|i| format!("    {i} abcdef"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = editor_with(&text);
        editor.set_view_size(80, 20);
        editor
    }

    fn place_top(editor: &mut Editor, top: usize) {
        editor.viewport.place_top(top, editor.line_count());
    }

    #[test]
    fn delete_to_viewport_top_keeps_column() {
        let mut editor = tall_editor();
        place_top(&mut editor, 40);
        editor.apply(Action::JumpTo(49, 8));
        editor.apply(Action::DeleteToViewportTop);
        assert_eq!(editor.text_lines()[40], "    51 abcdef");
        assert_eq!((editor.row(), editor.col()), (40, 8));
        assert_eq!(editor.register_text().lines().count(), 10);
    }

    #[test]
    fn delete_to_viewport_top_uses_count() {
        let mut editor = tall_editor();
        place_top(&mut editor, 40);
        editor.apply(Action::JumpTo(49, 0));
        push_digits(&mut editor, &[2]);
        editor.apply(Action::DeleteToViewportTop);
        assert_eq!(editor.text_lines()[40], "    41 abcdef");
        assert_eq!((editor.row(), editor.col()), (41, 0));
        assert_eq!(editor.register_text().lines().count(), 9);
    }

    #[test]
    fn delete_to_viewport_middle_straddles() {
        let mut editor = tall_editor();
        place_top(&mut editor, 45);
        editor.apply(Action::JumpTo(64, 8));
        editor.apply(Action::DeleteToViewportMiddle);
        assert_eq!(editor.text_lines()[54], "    66 abcdef");
        assert_eq!((editor.row(), editor.col()), (54, 8));
        let mut editor = tall_editor();
        place_top(&mut editor, 45);
        editor.apply(Action::JumpTo(45, 8));
        editor.apply(Action::DeleteToViewportMiddle);
        assert_eq!(editor.text_lines()[45], "    56 abcdef");
        assert_eq!((editor.row(), editor.col()), (45, 8));
    }

    #[test]
    fn delete_to_viewport_bottom_keeps_column() {
        let mut editor = tall_editor();
        place_top(&mut editor, 40);
        editor.apply(Action::JumpTo(49, 8));
        editor.apply(Action::DeleteToViewportBottom);
        assert_eq!(editor.text_lines()[49], "    61 abcdef");
        assert_eq!((editor.row(), editor.col()), (49, 8));
        assert_eq!(editor.register_text().lines().count(), 11);
    }

    #[test]
    fn yank_to_viewport_top_moves_to_first() {
        let mut editor = tall_editor();
        place_top(&mut editor, 40);
        editor.apply(Action::JumpTo(49, 8));
        editor.apply(Action::YankToViewportTop);
        assert_eq!(editor.text_lines().len(), 100);
        assert_eq!((editor.row(), editor.col()), (40, 8));
        assert_eq!(editor.register_text().lines().count(), 10);
    }

    #[test]
    fn yank_to_viewport_bottom_keeps_cursor() {
        let mut editor = tall_editor();
        place_top(&mut editor, 40);
        editor.apply(Action::JumpTo(49, 8));
        editor.apply(Action::YankToViewportBottom);
        assert_eq!((editor.row(), editor.col()), (49, 8));
        assert_eq!(editor.register_text().lines().count(), 11);
    }

    #[test]
    fn change_to_viewport_top_keeps_indent() {
        let mut editor = tall_editor();
        place_top(&mut editor, 40);
        editor.apply(Action::JumpTo(49, 8));
        editor.apply(Action::ChangeToViewportTop);
        assert_eq!(editor.text_lines()[40], "    ");
        assert_eq!((editor.row(), editor.col()), (40, 4));
        assert_eq!(editor.mode(), crate::mode::Mode::Insert);
        assert_eq!(editor.register_text().lines().count(), 10);
    }

    #[test]
    fn change_to_viewport_middle_straddles() {
        let mut editor = tall_editor();
        place_top(&mut editor, 45);
        editor.apply(Action::JumpTo(64, 0));
        editor.apply(Action::ChangeToViewportMiddle);
        assert_eq!(editor.text_lines()[54], "    ");
        assert_eq!((editor.row(), editor.col()), (54, 4));
        assert_eq!(editor.mode(), crate::mode::Mode::Insert);
    }

    #[test]
    fn change_to_last_line() {
        let mut editor = editor_with("a\nbb\ncc");
        editor.apply(Action::MoveDown);
        editor.apply(Action::ChangeToLastLine);
        assert_eq!(editor.mode(), crate::mode::Mode::Insert);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        assert_eq!(editor.text_lines()[0], "a");
        assert_eq!(editor.text_lines()[1], "X");
        assert_eq!(editor.line_count(), 2);
        assert_eq!((editor.row(), editor.col()), (1, 0));
    }

    #[test]
    fn change_to_first_line() {
        let mut editor = editor_with("a\nbb\ncc");
        editor.apply(Action::MoveDown);
        editor.apply(Action::MoveDown);
        editor.apply(Action::ChangeToFirstLine);
        editor.apply(Action::InsertChar('X'));
        editor.apply(Action::ExitInsert);
        assert_eq!(editor.text_lines()[0], "X");
        assert_eq!(editor.line_count(), 1);
        assert_eq!((editor.row(), editor.col()), (0, 0));
    }
}
