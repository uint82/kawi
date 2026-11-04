use super::Editor;
use crate::editor::indent::IndentConfig;
use crate::mode::Mode;
use crate::register::RegisterKind;
use crate::selection::Selection;

pub(super) struct BlockRect {
    pub top: usize,
    pub bottom: usize,
    pub left: usize,
    pub right: usize,
}

pub(super) struct BlockInsertPlan {
    top: usize,
    bottom: usize,
    screen_col: usize,
    cursor_col: usize,
    original_len: usize,
    skip_short: bool,
    ragged: bool,
}

enum BlockShift {
    Insert {
        row: usize,
        screen: usize,
        text: String,
    },
    Delete {
        row: usize,
        from: usize,
        to: usize,
    },
}

impl Editor {
    pub(super) fn enter_visual_block(&mut self) {
        if self.selection.is_none() {
            self.selection = Some(Selection::new(self.cursor.row(), self.cursor.col()));
        }
        self.mode = Mode::VisualBlock;
    }

    pub fn block_bounds(&self) -> Option<(usize, usize, usize, usize, bool)> {
        let rect = self.block_rect()?;
        Some((
            rect.top,
            rect.left,
            rect.bottom,
            rect.right,
            self.block_ragged(),
        ))
    }

    pub(super) fn block_rect(&self) -> Option<BlockRect> {
        if self.mode != Mode::VisualBlock {
            return None;
        }
        let selection = self.selection.as_ref()?;
        let (start, end) = (selection.start(), selection.end());
        let (start_left, start_right) = self.screen_span(start.0, start.1);
        let (end_left, end_right) = self.screen_span(end.0, end.1);
        Some(BlockRect {
            top: start.0,
            bottom: end.0,
            left: start_left.min(end_left),
            right: start_right.max(end_right),
        })
    }

    fn block_ragged(&self) -> bool {
        self.cursor.desired() == usize::MAX
    }

    fn block_row_text(&self, rect: &BlockRect, row: usize) -> String {
        if self.buffer.line_cow(row).is_none() {
            return String::new();
        }
        let width = self.screen_width(row);
        let target = rect.right.saturating_sub(rect.left).saturating_add(1);
        if self.block_ragged() {
            if width <= rect.left {
                return String::new();
            }
            return self.extract_screen_range(row, rect.left, width.saturating_sub(1));
        }
        if width < rect.left {
            return " ".repeat(target);
        }
        if width == rect.left {
            return String::new();
        }
        let right = rect.right.min(width.saturating_sub(1));
        self.extract_screen_range(row, rect.left, right)
    }

    pub(super) fn block_text(&self, rect: &BlockRect) -> String {
        (rect.top..=rect.bottom)
            .map(|row| self.block_row_text(rect, row))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn block_row_right(&self, rect: &BlockRect, row: usize) -> usize {
        if self.block_ragged() {
            self.screen_width(row).saturating_sub(1)
        } else {
            rect.right
        }
    }

    pub(super) fn delete_block_chars(&mut self, rect: &BlockRect) {
        for row in rect.top..=rect.bottom {
            let right = self.block_row_right(rect, row);
            self.delete_screen_range(row, rect.left, right);
        }
    }

    fn cursor_to_block_start(&mut self, rect: &BlockRect) {
        let col = self.char_col_at_screen(rect.top, rect.left);
        self.cursor.set_position(rect.top, col);
    }

    pub(super) fn delete_block(&mut self) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        let text = self.block_text(&rect);
        self.record_boundary_at(rect.top, rect.left);
        self.write_delete(
            RegisterKind::BlockWise,
            text,
            rect.bottom - rect.top + 1,
            false,
        );
        self.delete_block_chars(&rect);
        self.cursor_to_block_start(&rect);
    }

    pub(super) fn change_block(&mut self) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        let text = self.block_text(&rect);
        self.record_boundary_at(rect.top, rect.left);
        self.write_delete(
            RegisterKind::BlockWise,
            text,
            rect.bottom - rect.top + 1,
            false,
        );
        self.delete_block_chars(&rect);
        self.exit_visual();
        self.cursor_to_block_start(&rect);
        self.switch_to_insert();
        self.block_insert = Some(BlockInsertPlan {
            top: rect.top,
            bottom: rect.bottom,
            screen_col: rect.left,
            cursor_col: self.cursor.col(),
            original_len: self.buffer.line_len(rect.top),
            skip_short: true,
            ragged: false,
        });
    }

    pub(super) fn yank_block(&mut self) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        let text = self.block_text(&rect);
        self.write_yank(RegisterKind::BlockWise, text);
        self.cursor_to_block_start(&rect);
    }

    pub(super) fn replace_block(&mut self, replacement: char) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        self.record_boundary();
        for row in rect.top..=rect.bottom {
            let right = self.block_row_right(&rect, row);
            self.replace_screen_range(row, rect.left, right, replacement);
        }
        self.cursor_to_block_start(&rect);
    }

    pub(super) fn insert_block_before(&mut self) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        self.split_tab_at_screen(rect.top, rect.left);
        self.exit_visual();
        self.cursor_to_block_start(&rect);
        self.enter_insert();
        self.block_insert = Some(BlockInsertPlan {
            top: rect.top,
            bottom: rect.bottom,
            screen_col: rect.left,
            cursor_col: self.cursor.col(),
            original_len: self.buffer.line_len(rect.top),
            skip_short: true,
            ragged: false,
        });
    }

    pub(super) fn append_block_after(&mut self) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        let ragged = self.block_ragged();
        let screen_col = if ragged {
            self.screen_width(rect.top)
        } else {
            rect.right.saturating_add(1)
        };
        self.exit_visual();
        if !ragged {
            self.insert_at_screen(rect.top, screen_col, "");
        }
        let col = self.char_col_at_screen(rect.top, screen_col);
        self.cursor.set_position(rect.top, col);
        self.enter_insert();
        self.block_insert = Some(BlockInsertPlan {
            top: rect.top,
            bottom: rect.bottom,
            screen_col,
            cursor_col: col,
            original_len: self.buffer.line_len(rect.top),
            skip_short: false,
            ragged,
        });
    }

    pub(super) fn replicate_block_insert(&mut self) {
        let Some(plan) = self.block_insert.take() else {
            return;
        };
        if self.cursor.row() != plan.top || plan.top == plan.bottom {
            return;
        }
        // TODO: skip replication when insert mode ended with Ctrl-C, like nvim
        let new_len = self.buffer.line_len(plan.top);
        let inserted_len = new_len.saturating_sub(plan.original_len);
        if inserted_len == 0 {
            return;
        }
        let start = self.char_col_at_screen(plan.top, plan.screen_col);
        let text = match self.buffer.line_cow(plan.top) {
            Some(line) => line
                .chars()
                .skip(start)
                .take(inserted_len)
                .collect::<String>(),
            None => return,
        };
        for row in (plan.top + 1)..=plan.bottom {
            if plan.ragged {
                let col = self.buffer.line_len(row);
                self.buffer.paste_before(row, col, &text);
                continue;
            }
            if plan.skip_short && self.screen_width(row) < plan.screen_col {
                continue;
            }
            self.insert_at_screen(row, plan.screen_col, &text);
        }
        self.cursor.set_position(plan.top, plan.cursor_col);
    }

    pub(super) fn swap_block_corners(&mut self) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        let Some(selection) = self.selection else {
            return;
        };
        let anchor_row = selection.anchor().0;
        let old_row = self.cursor.row();
        let old_col = self.cursor.col();
        let anchor_left = self.char_col_at_screen(anchor_row, rect.left);
        let anchor_right = self.char_col_at_screen(anchor_row, rect.right);
        let head_left = self.char_col_at_screen(old_row, rect.left);
        let head_right = self.char_col_at_screen(old_row, rect.right);
        if head_right == old_col {
            self.selection = Some(Selection::from_ends(
                (anchor_row, anchor_right),
                (old_row, head_left),
            ));
            self.cursor.set_position(old_row, head_left);
        } else {
            self.selection = Some(Selection::from_ends(
                (anchor_row, anchor_left),
                (old_row, head_right),
            ));
            self.cursor.set_position(old_row, head_right);
        }
    }

    pub(super) fn shift_block(&mut self, right: bool) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        let shifts = self.block_shifts(&rect, right);
        if shifts.is_empty() {
            return;
        }
        self.record_boundary_at(rect.top, rect.left);
        for shift in shifts {
            self.apply_block_shift(shift);
        }
        self.cursor.set_position(rect.top, rect.left);
    }

    fn block_shifts(&self, rect: &BlockRect, right: bool) -> Vec<BlockShift> {
        let indent = self.effective_indent();
        let width = indent.shift_unit();
        if width == 0 {
            return Vec::new();
        }
        (rect.top..=rect.bottom)
            .filter_map(|row| self.block_shift(rect, row, right, &indent, width))
            .collect()
    }

    fn block_shift(
        &self,
        rect: &BlockRect,
        row: usize,
        right: bool,
        indent: &IndentConfig,
        width: usize,
    ) -> Option<BlockShift> {
        if self.screen_width(row) < rect.left {
            return None;
        }
        if right {
            let text = indent_text(rect.left, width, indent.expandtab, indent.tabstop);
            return (!text.is_empty()).then_some(BlockShift::Insert {
                row,
                screen: rect.left,
                text,
            });
        }
        let left = self.char_col_at_screen(row, rect.left);
        let from = self.block_unindent_start(row, left, width, indent.tabstop)?;
        Some(BlockShift::Delete {
            row,
            from,
            to: left.saturating_sub(1),
        })
    }

    fn block_unindent_start(
        &self,
        row: usize,
        col: usize,
        width: usize,
        tabstop: usize,
    ) -> Option<usize> {
        let mut removed = 0;
        let mut from = col;
        while from > 0 && removed < width {
            let Some(ch) = self.buffer.char_at(row, from - 1) else {
                break;
            };
            if ch != ' ' && ch != '\t' {
                break;
            }
            removed += self.buffer.screen_col(row, from, tabstop)
                - self.buffer.screen_col(row, from - 1, tabstop);
            from -= 1;
        }
        (from < col).then_some(from)
    }

    fn apply_block_shift(&mut self, shift: BlockShift) {
        match shift {
            BlockShift::Insert { row, screen, text } => {
                self.insert_at_screen(row, screen, &text);
            }
            BlockShift::Delete { row, from, to } => {
                self.buffer.delete_range(row, from, row, to);
            }
        }
    }

    pub(super) fn shift_block_case(&mut self, shift: fn(char) -> char) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        self.record_boundary();
        for row in rect.top..=rect.bottom {
            let width = self.screen_width(row);
            if width <= rect.left {
                continue;
            }
            let start = self.char_col_at_screen(row, rect.left);
            let right = self
                .block_row_right(&rect, row)
                .min(width.saturating_sub(1));
            let end = self.char_col_at_screen(row, right);
            for col in start..=end {
                let Some(ch) = self.buffer.char_at(row, col) else {
                    continue;
                };
                let mapped = shift(ch);
                if mapped != ch {
                    self.replace_text_at(row, col, 1, &mapped.to_string());
                }
            }
        }
        self.cursor_to_block_start(&rect);
    }

    pub(super) fn replace_selection(&mut self, replacement: char) {
        if self.mode == Mode::VisualBlock {
            self.replace_block(replacement);
        }
    }
}

fn indent_text(screen_start: usize, width: usize, expandtab: bool, tabstop: usize) -> String {
    let mut text = String::new();
    let mut at = screen_start;
    let target = screen_start.saturating_add(width);
    let tabstop = tabstop.max(1);
    while at < target {
        let next_tab = at / tabstop * tabstop + tabstop;
        if !expandtab && next_tab <= target {
            text.push('\t');
            at = next_tab;
        } else {
            text.push(' ');
            at += 1;
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn enter_block(editor: &mut Editor, row: usize, col: usize) {
        editor.apply(Action::JumpTo(row, col));
        editor.apply(Action::EnterVisualBlock);
    }

    fn paste_after(editor: &mut Editor) {
        editor.apply(Action::PasteAfter);
    }

    #[test]
    fn enter_block_keeps_cursor_and_mode() {
        let mut e = editor_with("abcdef\ngh\nijkl");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        assert_eq!(e.mode(), Mode::VisualBlock);
        assert_eq!(e.block_bounds(), Some((0, 2, 1, 2, false)));
        assert_eq!((e.row(), e.col()), (1, 2));
    }

    #[test]
    fn block_yank_stores_blockwise_text() {
        let mut e = editor_with("abcdef\ngh\nijklmn");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::YankSelection);
        assert_eq!(e.register_kind(), RegisterKind::BlockWise);
        assert_eq!(e.register_text(), "c\n\nk");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn block_yank_pads_missing_lines() {
        let mut e = editor_with("abcdef\nxy\nabcdef");
        enter_block(&mut e, 0, 4);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::YankSelection);
        assert_eq!(e.register_text(), "e\n \ne");
    }

    #[test]
    fn block_yank_partial_lines_keep_text_only() {
        let mut e = editor_with("abcdef\nabcde\nabcdef");
        enter_block(&mut e, 0, 4);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        assert_eq!(e.register_text(), "ef\ne\nef");
    }

    #[test]
    fn block_yank_ragged_uses_line_ends() {
        let mut e = editor_with("abcdef\ngh");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::YankSelection);
        assert_eq!(e.register_kind(), RegisterKind::BlockWise);
        assert_eq!(e.register_text(), "abcdef\ngh");
    }

    #[test]
    fn block_delete_removes_rectangle() {
        let mut e = editor_with("abc\nabcdef\nabc");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteSelection);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["ac", "acdef", "ac"]);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn block_delete_with_short_corner_extends_left() {
        let mut e = editor_with("abc\na\n\n");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteSelection);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["", "", ""]);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn block_change_replicates_typed_text() {
        let mut e = editor_with("abc\na\n\n");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::ChangeSelection);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["XY", "XY", "XY"]);
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn block_insert_before_skips_short_rows() {
        let mut e = editor_with("abcde\nab\na\nabc");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::InsertBeforeSelection);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["abXYcde", "abXY", "a", "abXYc"]);
    }

    #[test]
    fn block_append_pads_short_rows() {
        let mut e = editor_with("abc\na\n\n");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::InsertAfterSelection);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["abcXY", "a  XY", "   XY"]);
    }

    #[test]
    fn block_replace_replaces_existing_chars_only() {
        let mut e = editor_with("abcde\nab\na\nabc");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::ReplaceSelection('Z'));
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["abZZe", "ab", "a", "abZ"]);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn block_replace_is_one_undo_step() {
        let mut e = editor_with("abc\ndef");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::ReplaceSelection('X'));
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["Xbc", "Xef"]);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["abc", "def"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn block_paste_count_repeats_each_row() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr\nstuvwx");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        e.apply(Action::JumpTo(2, 0));
        e.apply(Action::PushCount(2));
        paste_after(&mut e);
        assert_eq!(
            e.text_lines(),
            vec!["abcdef", "ghijkl", "mababnopqr", "sghghtuvwx"]
        );
        assert_eq!((e.row(), e.col()), (2, 1));
    }

    #[test]
    fn block_paste_over_charwise_register_repeats_each_row() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr");
        e.yank_text("XY");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::PasteOverSelection);
        assert_eq!(e.text_lines(), vec!["aXYdef", "gXYjkl", "mnopqr"]);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn block_paste_over_multiline_charwise_register_inserts_once() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr");
        e.yank_text("XY\nZW");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::PasteOverSelection);
        assert_eq!(e.text_lines(), vec!["aXY", "ZWdef", "gjkl", "mnopqr"]);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn block_paste_over_linewise_register_inserts_below_block() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr\nstuvwx");
        e.apply(Action::JumpTo(2, 0));
        e.apply(Action::YankLine);
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::PasteOverSelection);
        assert_eq!(
            e.text_lines(),
            vec!["adef", "gjkl", "mnopqr", "mnopqr", "stuvwx"]
        );
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn block_paste_over_blockwise_register_repeats_count() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr\nstuvwx");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        e.apply(Action::ExitVisual);
        enter_block(&mut e, 2, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::PushCount(2));
        e.apply(Action::PasteOverSelection);
        assert_eq!(
            e.text_lines(),
            vec!["abcdef", "ghijkl", "mababpqr", "sghghvwx"]
        );
        assert_eq!((e.row(), e.col()), (2, 1));
    }

    #[test]
    fn block_paste_over_charwise_register_repeats_count() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr");
        e.yank_text("XY");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::PushCount(2));
        e.apply(Action::PasteOverSelection);
        assert_eq!(e.text_lines(), vec!["aXYXYef", "gXYXYkl", "mnopqr"]);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn block_paste_over_multiline_charwise_register_repeats_count() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr");
        e.yank_text("XY\nZW");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::PushCount(2));
        e.apply(Action::PasteOverSelection);
        assert_eq!(e.text_lines(), vec!["aXY", "ZWXY", "ZWef", "gkl", "mnopqr"]);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn block_paste_over_linewise_register_repeats_count() {
        let mut e = editor_with("abcdef\nghijkl\nmnopqr\nstuvwx");
        e.apply(Action::JumpTo(2, 0));
        e.apply(Action::YankLine);
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::PushCount(2));
        e.apply(Action::PasteOverSelection);
        assert_eq!(
            e.text_lines(),
            vec!["adef", "gjkl", "mnopqr", "mnopqr", "mnopqr", "stuvwx"]
        );
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn block_paste_after_inserts_at_next_column() {
        let mut e = editor_with("abcdef\ngh\nijklmn\nopqrst\nuvwxyz\n123456\n7890ab");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        e.apply(Action::JumpTo(3, 0));
        paste_after(&mut e);
        assert_eq!(
            e.text_lines(),
            vec!["abcdef", "gh", "ijklmn", "oabpqrst", "ughvwxyz", "1ij23456", "7890ab"]
        );
        assert_eq!((e.row(), e.col()), (3, 1));
    }

    #[test]
    fn block_paste_before_inserts_at_cursor() {
        let mut e = editor_with("abcdef\ngh\nijklmn\nopqrst\nuvwxyz\n123456\n7890ab");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::PasteBefore);
        assert_eq!(e.text_lines()[3], "abopqrst");
        assert_eq!(e.text_lines()[4], "ghuvwxyz");
        assert_eq!(e.text_lines()[5], "ij123456");
        assert_eq!((e.row(), e.col()), (3, 0));
    }

    #[test]
    fn block_paste_before_appends_missing_rows() {
        let mut e = editor_with("abcdef\nghijkl\nmn\nopqrst\nuvwxyz");
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::YankSelection);
        e.apply(Action::GotoLastLine);
        e.apply(Action::MoveToLineStart);
        e.apply(Action::PasteBefore);
        assert_eq!(
            e.text_lines(),
            vec!["abcdef", "ghijkl", "mn", "opqrst", "cuvwxyz", "i", "", "q"]
        );
    }

    #[test]
    fn block_o_swaps_ends_and_shift_o_swaps_corners() {
        let mut e = editor_with("abcdef\ngh\nijklmn");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::SwapSelectionEnds);
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::SwapSelectionEnds);
        assert_eq!((e.row(), e.col()), (2, 2));
        e.apply(Action::SwapBlockCorners);
        assert_eq!((e.row(), e.col()), (2, 0));
        e.apply(Action::SwapBlockCorners);
        assert_eq!((e.row(), e.col()), (2, 2));
    }

    #[test]
    fn block_shift_right_indents_rows() {
        let mut e = editor_with("    abcdef\n    gh\n  ijklmn");
        e.set_shiftwidth(2);
        e.set_expandtab(true);
        enter_block(&mut e, 0, 4);
        e.apply(Action::MoveDown);
        e.apply(Action::ShiftRight);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["      abcdef", "      gh", "  ijklmn"]);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn block_shift_left_removes_indent_before_block() {
        let mut e = editor_with("    abcdef\n    gh");
        e.set_shiftwidth(2);
        e.set_expandtab(true);
        enter_block(&mut e, 0, 4);
        e.apply(Action::MoveDown);
        e.apply(Action::ShiftLeft);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["  abcdef", "  gh"]);
    }

    #[test]
    fn block_shift_right_is_one_undo_step() {
        let mut e = editor_with("  abc\n  def");
        e.set_shiftwidth(2);
        e.set_expandtab(true);
        enter_block(&mut e, 0, 2);
        e.apply(Action::MoveDown);
        e.apply(Action::ShiftRight);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["    abc", "    def"]);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["  abc", "  def"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn block_shift_left_is_one_undo_step() {
        let mut e = editor_with("    abc\n    def");
        e.set_shiftwidth(2);
        e.set_expandtab(true);
        enter_block(&mut e, 0, 4);
        e.apply(Action::MoveDown);
        e.apply(Action::ShiftLeft);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["  abc", "  def"]);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["    abc", "    def"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn block_shift_left_without_indent_keeps_the_buffer_clean() {
        let mut e = editor_with("abc\ndef");
        enter_block(&mut e, 0, 0);
        e.apply(Action::MoveDown);
        e.apply(Action::ShiftLeft);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["abc", "def"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn block_toggle_case_changes_block_chars() {
        let mut e = editor_with("abcdef\ngh\nijklmn");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::ToggleCase);
        e.apply(Action::ExitVisual);
        assert_eq!(e.text_lines(), vec!["aBcdef", "gH", "iJklmn"]);
    }

    #[test]
    fn block_toggle_case_is_one_undo_step() {
        let mut e = editor_with("abcdef\ngh\nijklmn");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::ToggleCase);
        e.apply(Action::ExitVisual);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["abcdef", "gh", "ijklmn"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn block_reselect_restores_block() {
        let mut e = editor_with("abcdef\ngh\nijklmn");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::CancelVisual);
        e.apply(Action::ReselectVisual);
        assert_eq!(e.mode(), Mode::VisualBlock);
        assert_eq!(e.block_bounds(), Some((0, 1, 1, 1, false)));
    }

    #[test]
    fn block_toggle_modes_keep_cursor() {
        let mut e = editor_with("abcdef\ngh");
        enter_block(&mut e, 0, 1);
        e.apply(Action::MoveDown);
        e.apply(Action::EnterVisual);
        assert_eq!(e.mode(), Mode::Visual);
        e.apply(Action::EnterVisualBlock);
        assert_eq!(e.mode(), Mode::VisualBlock);
        e.apply(Action::EnterVisualLine);
        assert_eq!(e.mode(), Mode::VisualLine);
    }
}
