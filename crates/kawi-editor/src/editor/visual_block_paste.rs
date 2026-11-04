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
        self.cursor.set_position(rect.top, rect.left);
        self.pad_row_to(rect.top, rect.left);
        self.block_paste(register, count, true, false);
    }

    fn paste_charwise_over_rect(&mut self, rect: &BlockRect, text: &str, count: usize) {
        let text = text.repeat(count);
        for row in rect.top..=rect.bottom {
            self.pad_row_to(row, rect.left);
            self.buffer.paste_before(row, rect.left, &text);
        }
        let last_col = rect
            .left
            .saturating_add(text.chars().count())
            .saturating_sub(1);
        let max_col = self.buffer.line_len(rect.top).saturating_sub(1);
        self.cursor.set_position(rect.top, last_col.min(max_col));
    }

    fn paste_text_over_rect(&mut self, rect: &BlockRect, text: &str) {
        self.pad_row_to(rect.top, rect.left);
        self.buffer.paste_before(rect.top, rect.left, text);
        self.cursor.set_position(rect.top, rect.left);
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
        let col = if before {
            self.cursor.col()
        } else {
            self.cursor.col().saturating_add(1)
        };
        self.change_origin_override = Some((row, col));
        let mut last_appended_empty = false;
        for (offset, text) in lines.iter().enumerate() {
            let target = row.saturating_add(offset);
            if target >= self.buffer.line_count() {
                let mut line = " ".repeat(col);
                line.push_str(text);
                self.buffer
                    .paste_before(target.saturating_sub(1), usize::MAX, "\n");
                self.buffer.paste_before(target, usize::MAX, &line);
                last_appended_empty = text.is_empty() && offset + 1 == lines.len();
                continue;
            }
            last_appended_empty = false;
            self.pad_row_to(target, col);
            self.buffer.paste_before(target, col, text);
        }
        if last_appended_empty {
            self.buffer
                .paste_before(self.buffer.len(), usize::MAX, "\n");
        }
        self.cursor.set_position(row, col);
        if advance {
            let width = lines
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0);
            let last_row = self.buffer.line_count().saturating_sub(1);
            let target_row = row
                .saturating_add(lines.len().saturating_sub(1))
                .min(last_row);
            let len = self.buffer.line_len(target_row);
            let target_col = col.saturating_add(width).min(len.saturating_sub(1));
            self.cursor.set_position(target_row, target_col);
        }
    }
}
