use super::visual_block::BlockRect;
use super::Editor;
use crate::register::{Register, RegisterKind};

impl Editor {
    pub(super) fn paste_over_block(&mut self, register: &Register) {
        let Some(rect) = self.block_rect() else {
            return;
        };
        let count = self.take_count();
        let text = self.block_text(&rect);
        self.record_boundary_at(rect.top, rect.left);
        self.write_delete(
            RegisterKind::BlockWise,
            text,
            rect.bottom - rect.top + 1,
            false,
        );
        self.delete_block_chars(&rect);
        match register.kind() {
            RegisterKind::BlockWise => self.paste_block_over_rect(&rect, register, count),
            RegisterKind::CharWise if !register.text().contains('\n') => {
                self.paste_charwise_over_rect(&rect, register.text(), count);
            }
            RegisterKind::CharWise => {
                let text = register.text().repeat(count);
                self.paste_text_over_rect(&rect, &text);
            }
            RegisterKind::LineWise => self.paste_lines_after_rect(&rect, register, count),
        }
    }

    fn paste_block_over_rect(&mut self, rect: &BlockRect, register: &Register, count: usize) {
        self.insert_at_screen(rect.top, rect.left, "");
        let col = self.char_col_at_screen(rect.top, rect.left);
        self.cursor.set_position(rect.top, col);
        self.block_paste(register, count, true, false);
    }

    fn paste_charwise_over_rect(&mut self, rect: &BlockRect, text: &str, count: usize) {
        let text = text.repeat(count);
        for row in rect.top..=rect.bottom {
            self.insert_at_screen(row, rect.left, &text);
        }
        let width = self.screen_text_width(&text, rect.left);
        let last_screen = rect.left.saturating_add(width).saturating_sub(1);
        let col = self.char_col_at_screen(rect.top, last_screen);
        self.cursor.set_position(rect.top, col);
    }

    fn paste_text_over_rect(&mut self, rect: &BlockRect, text: &str) {
        self.insert_at_screen(rect.top, rect.left, "");
        let col = self.char_col_at_screen(rect.top, rect.left);
        self.buffer.paste_before(rect.top, col, text);
        self.cursor.set_position(rect.top, col);
    }

    fn paste_lines_after_rect(&mut self, rect: &BlockRect, register: &Register, count: usize) {
        self.cursor.set_position(rect.bottom, 0);
        self.put_register(register, count, false, false);
    }

    pub(super) fn block_paste(
        &mut self,
        register: &Register,
        count: usize,
        before: bool,
        advance: bool,
    ) {
        let repetitions = count.max(1);
        let lines: Vec<String> = register
            .lines()
            .into_iter()
            .map(|line| line.repeat(repetitions))
            .collect();
        let row = self.cursor.row();
        let screen = if before {
            self.buffer
                .screen_col(row, self.cursor.col(), self.tabstop())
        } else {
            self.buffer
                .screen_col(row, self.cursor.col().saturating_add(1), self.tabstop())
        };
        let col = self.char_col_at_screen(row, screen);
        self.change_origin_override = Some((row, col));
        let mut last_appended_empty = false;
        for (offset, text) in lines.iter().enumerate() {
            let target = row.saturating_add(offset);
            if target >= self.buffer.line_count() {
                let mut line = " ".repeat(screen);
                line.push_str(text);
                self.buffer
                    .paste_before(target.saturating_sub(1), usize::MAX, "\n");
                self.buffer.paste_before(target, usize::MAX, &line);
                last_appended_empty = text.is_empty() && offset + 1 == lines.len();
                continue;
            }
            last_appended_empty = false;
            self.insert_at_screen(target, screen, text);
        }
        if last_appended_empty {
            self.buffer
                .paste_before(self.buffer.len(), usize::MAX, "\n");
        }
        self.cursor.set_position(row, col);
        if advance {
            let width = lines
                .iter()
                .map(|line| self.screen_text_width(line, screen))
                .max()
                .unwrap_or(0);
            let last_row = self.buffer.line_count().saturating_sub(1);
            let target_row = row
                .saturating_add(lines.len().saturating_sub(1))
                .min(last_row);
            let target_screen = screen.saturating_add(width).saturating_sub(1);
            let target_col = self.char_col_at_screen(target_row, target_screen);
            self.cursor.set_position(target_row, target_col);
        }
    }
}
