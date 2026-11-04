use super::Editor;
use kawi_buffer::tab_width;

impl Editor {
    pub(super) fn screen_width(&self, row: usize) -> usize {
        let len = self.buffer.line_len(row);
        self.buffer.screen_col(row, len, self.tabstop())
    }

    pub(super) fn screen_span(&self, row: usize, col: usize) -> (usize, usize) {
        let (start, end) = self.buffer.screen_span(row, col, self.tabstop());
        if col >= self.buffer.line_len(row) {
            return (start, start);
        }
        (start, end.saturating_sub(1).max(start))
    }

    pub(super) fn char_col_at_screen(&self, row: usize, screen: usize) -> usize {
        let len = self.buffer.line_len(row);
        if len == 0 || self.screen_width(row) <= screen {
            return len;
        }
        let tabstop = self.tabstop();
        let mut position = 0;
        if let Some(line) = self.buffer.line_cow(row) {
            for (index, ch) in line.chars().enumerate() {
                let width = tab_width(ch, position, tabstop);
                if screen < position.saturating_add(width) {
                    return index;
                }
                position = position.saturating_add(width);
            }
        }
        len
    }

    pub(super) fn extract_screen_range(&self, row: usize, left: usize, right: usize) -> String {
        let Some(line) = self.buffer.line_cow(row) else {
            return String::new();
        };
        let tabstop = self.tabstop();
        let target = right.saturating_sub(left).saturating_add(1);
        let mut text = String::new();
        let mut covered: usize = 0;
        let mut position = 0;
        for ch in line.chars() {
            let width = tab_width(ch, position, tabstop);
            let start = position;
            let end = position.saturating_add(width).saturating_sub(1);
            position = position.saturating_add(width);
            if end < left {
                continue;
            }
            if start > right {
                break;
            }
            let overlap_start = start.max(left);
            let overlap_end = end.min(right);
            let cells = overlap_end.saturating_sub(overlap_start).saturating_add(1);
            if overlap_start == start && overlap_end == end {
                text.push(ch);
            } else {
                text.push_str(&" ".repeat(cells));
            }
            covered = covered.saturating_add(cells);
        }
        if covered < target {
            text.push_str(&" ".repeat(target.saturating_sub(covered)));
        }
        text
    }

    pub(super) fn delete_screen_range(&mut self, row: usize, left: usize, right: usize) {
        let Some(line) = self.buffer.line_cow(row).map(|line| line.into_owned()) else {
            return;
        };
        let width = self.screen_width(row);
        if width <= left {
            return;
        }
        let right = right.min(width.saturating_sub(1));
        let tabstop = self.tabstop();
        let mut text = String::new();
        let mut position = 0;
        for ch in line.chars() {
            let char_width = tab_width(ch, position, tabstop);
            let start = position;
            let end = position.saturating_add(char_width).saturating_sub(1);
            position = position.saturating_add(char_width);
            if end < left || start > right {
                text.push(ch);
                continue;
            }
            if ch == '\t' {
                if start < left {
                    text.push_str(&" ".repeat(left.saturating_sub(start)));
                }
                if end > right {
                    text.push_str(&" ".repeat(end.saturating_sub(right)));
                }
            }
        }
        self.rewrite_line(row, &line, text);
    }

    pub(super) fn replace_screen_range(
        &mut self,
        row: usize,
        left: usize,
        right: usize,
        replacement: char,
    ) {
        let Some(line) = self.buffer.line_cow(row).map(|line| line.into_owned()) else {
            return;
        };
        let width = self.screen_width(row);
        if width <= left {
            return;
        }
        let right = right.min(width.saturating_sub(1));
        let tabstop = self.tabstop();
        let mut text = String::new();
        let mut position = 0;
        for ch in line.chars() {
            let char_width = tab_width(ch, position, tabstop);
            let start = position;
            let end = position.saturating_add(char_width).saturating_sub(1);
            position = position.saturating_add(char_width);
            if end < left || start > right {
                text.push(ch);
                continue;
            }
            if start < left {
                text.push_str(&" ".repeat(left.saturating_sub(start)));
            }
            let cells = end
                .min(right)
                .saturating_sub(start.max(left))
                .saturating_add(1);
            text.extend(std::iter::repeat_n(replacement, cells));
            if end > right {
                text.push_str(&" ".repeat(end.saturating_sub(right)));
            }
        }
        self.rewrite_line(row, &line, text);
    }

    pub(super) fn split_tab_at_screen(&mut self, row: usize, screen: usize) {
        let Some(line) = self.buffer.line_cow(row).map(|line| line.into_owned()) else {
            return;
        };
        let tabstop = self.tabstop();
        let mut text = String::new();
        let mut position = 0;
        for ch in line.chars() {
            let char_width = tab_width(ch, position, tabstop);
            let start = position;
            let end = position.saturating_add(char_width).saturating_sub(1);
            position = position.saturating_add(char_width);
            if ch == '\t' && start < screen && screen <= end {
                text.push_str(&" ".repeat(char_width));
            } else {
                text.push(ch);
            }
        }
        self.rewrite_line(row, &line, text);
    }

    pub(super) fn screen_text_width(&self, text: &str, screen: usize) -> usize {
        let tabstop = self.tabstop();
        let mut position = screen;
        for ch in text.chars() {
            position = position.saturating_add(tab_width(ch, position, tabstop));
        }
        position.saturating_sub(screen)
    }

    pub(super) fn insert_at_screen(&mut self, row: usize, screen: usize, insert: &str) {
        let Some(line) = self.buffer.line_cow(row).map(|line| line.into_owned()) else {
            return;
        };
        let width = self.screen_width(row);
        let tabstop = self.tabstop();
        let mut text = String::new();
        let mut position = 0;
        let mut inserted = false;
        for ch in line.chars() {
            let char_width = tab_width(ch, position, tabstop);
            let start = position;
            let end = position.saturating_add(char_width).saturating_sub(1);
            position = position.saturating_add(char_width);
            if !inserted && screen <= start {
                text.push_str(insert);
                inserted = true;
            }
            if inserted {
                text.push(ch);
                continue;
            }
            if screen <= end {
                if ch == '\t' {
                    text.push_str(&" ".repeat(screen.saturating_sub(start)));
                    text.push_str(insert);
                    text.push_str(&" ".repeat(end.saturating_sub(screen).saturating_add(1)));
                } else {
                    text.push_str(insert);
                    text.push(ch);
                }
                inserted = true;
            } else {
                text.push(ch);
            }
        }
        if !inserted {
            if width < screen {
                text.push_str(&" ".repeat(screen.saturating_sub(width)));
            }
            text.push_str(insert);
        }
        self.rewrite_line(row, &line, text);
    }

    fn rewrite_line(&mut self, row: usize, old: &str, new: String) {
        if old != new {
            self.buffer.replace_lines(row, row, &new);
        }
    }
}
