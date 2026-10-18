use super::Editor;

impl Editor {
    pub fn set_number(&mut self, number: bool) {
        self.number = number;
    }

    pub fn set_relativenumber(&mut self, relativenumber: bool) {
        self.relativenumber = relativenumber;
    }

    pub fn set_numberwidth(&mut self, numberwidth: usize) {
        self.numberwidth = numberwidth;
    }

    pub fn number(&self) -> bool {
        self.number
    }

    pub fn relativenumber(&self) -> bool {
        self.relativenumber
    }

    pub fn numberwidth(&self) -> usize {
        self.numberwidth
    }

    pub fn gutter_width(&self) -> usize {
        if self.numbers_visible() {
            self.number_column_width().saturating_add(1)
        } else {
            0
        }
    }

    pub fn gutter_lines(&self, first: usize, count: usize) -> Vec<String> {
        (first..first.saturating_add(count))
            .map(|row| self.gutter_line(row))
            .collect()
    }

    pub fn cursor_wcol(&self) -> usize {
        self.gutter_width().saturating_add(self.cursor_screen_col())
    }

    pub fn buffer_col_at_screen(&self, row: usize, screen: usize) -> usize {
        let tabstop = self.effective_indent().tabstop;
        let col = screen.saturating_sub(self.gutter_width());
        self.buffer.char_col(row, col, tabstop)
    }

    fn gutter_line(&self, row: usize) -> String {
        if !self.numbers_visible() {
            return String::new();
        }
        let width = self.number_column_width();
        match self.display_number(row) {
            Some((number, false)) => format!("{number:>width$} "),
            Some((number, true)) => format!("{number:<width$} "),
            None => " ".repeat(width.saturating_add(1)),
        }
    }

    fn display_number(&self, row: usize) -> Option<(usize, bool)> {
        if row >= self.buffer.line_count() {
            return None;
        }
        if self.number && !self.relativenumber {
            return Some((row.saturating_add(1), false));
        }
        let distance = row.abs_diff(self.cursor.row());
        if distance == 0 && self.number {
            return Some((row.saturating_add(1), true));
        }
        Some((distance, false))
    }

    fn numbers_visible(&self) -> bool {
        self.number || self.relativenumber
    }

    fn number_column_width(&self) -> usize {
        let reference = if self.relativenumber && !self.number {
            self.view_height()
        } else {
            self.buffer.line_count()
        };
        digit_count(reference).max(self.numberwidth.saturating_sub(1))
    }
}

fn digit_count(value: usize) -> usize {
    let mut digits = 1;
    let mut rest = value;
    while rest >= 10 {
        rest /= 10;
        digits += 1;
    }
    digits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn three_lines() -> Editor {
        editor_with("\tello world\nsecond\nthird")
    }

    #[test]
    fn gutter_hidden_by_default() {
        let e = three_lines();
        assert_eq!(e.gutter_width(), 0);
        assert_eq!(e.gutter_lines(0, 3), vec!["", "", ""]);
    }

    #[test]
    fn number_shows_padded_numbers() {
        let mut e = three_lines();
        e.apply(Action::JumpTo(1, 0));
        e.set_number(true);
        assert_eq!(e.gutter_width(), 4);
        assert_eq!(e.gutter_lines(0, 3), vec!["  1 ", "  2 ", "  3 "]);
    }

    #[test]
    fn relative_number_shows_distance_from_cursor() {
        let mut e = three_lines();
        e.apply(Action::JumpTo(1, 0));
        e.set_relativenumber(true);
        assert_eq!(e.gutter_lines(0, 3), vec!["  1 ", "  0 ", "  1 "]);
    }

    #[test]
    fn number_with_relative_number_shows_absolute_on_cursor_line() {
        let mut e = three_lines();
        e.apply(Action::JumpTo(1, 0));
        e.set_number(true);
        e.set_relativenumber(true);
        assert_eq!(e.gutter_lines(0, 3), vec!["  1 ", "2   ", "  1 "]);
    }

    #[test]
    fn numberwidth_sets_minimal_column_width() {
        let mut e = three_lines();
        e.set_number(true);
        e.set_numberwidth(1);
        assert_eq!(e.gutter_width(), 2);
        assert_eq!(e.gutter_lines(0, 3), vec!["1 ", "2 ", "3 "]);
        e.set_numberwidth(6);
        assert_eq!(e.gutter_width(), 6);
        assert_eq!(e.gutter_lines(0, 1), vec!["    1 "]);
    }

    #[test]
    fn width_grows_with_line_count() {
        let text = (1..=12)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let mut e = editor_with(&text);
        e.set_number(true);
        assert_eq!(e.gutter_width(), 4);
        assert_eq!(e.gutter_lines(9, 2), vec![" 10 ", " 11 "]);
    }

    #[test]
    fn rows_past_last_line_are_blank() {
        let mut e = three_lines();
        e.set_number(true);
        assert_eq!(e.gutter_lines(2, 2), vec!["  3 ", "    "]);
    }

    #[test]
    fn cursor_wcol_adds_gutter_to_tab_column() {
        let mut e = editor_with("\tello world");
        e.apply(Action::JumpTo(0, 0));
        assert_eq!(e.cursor_screen_col(), 7);
        assert_eq!(e.cursor_wcol(), 7);
        e.set_number(true);
        assert_eq!(e.cursor_wcol(), 11);
    }

    #[test]
    fn buffer_col_at_screen_skips_gutter_and_snaps_tabs() {
        let mut e = editor_with("\tello world");
        e.set_number(true);
        assert_eq!(e.buffer_col_at_screen(0, 0), 0);
        assert_eq!(e.buffer_col_at_screen(0, 3), 0);
        assert_eq!(e.buffer_col_at_screen(0, 4), 0);
        assert_eq!(e.buffer_col_at_screen(0, 11), 0);
        assert_eq!(e.buffer_col_at_screen(0, 12), 1);
        assert_eq!(e.buffer_col_at_screen(0, 99), 10);
    }
}
