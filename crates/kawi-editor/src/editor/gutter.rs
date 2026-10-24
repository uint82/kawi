use super::{Editor, WindowOptions};

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

    pub fn set_signcolumn_width(&mut self, width: usize) {
        self.signcolumn_width = width;
    }

    pub fn signcolumn_width(&self) -> usize {
        self.signcolumn_width
    }

    pub fn gutter_width(&self) -> usize {
        let options = self.window_options();
        let numbers = if numbers_visible(options) {
            number_column_width_for(options, self.buffer.line_count(), self.view_height())
                .saturating_add(1)
        } else {
            0
        };
        self.signcolumn_width.saturating_add(numbers)
    }

    pub fn gutter_lines(&self, first: usize, count: usize) -> Vec<String> {
        self.gutter_lines_for(
            self.buffer.line_count(),
            self.cursor.row(),
            first,
            count,
            self.view_height(),
            self.window_options(),
        )
    }

    pub fn buffer_gutter_lines(
        &self,
        number: usize,
        first: usize,
        count: usize,
        cursor_row: usize,
        height: usize,
        options: WindowOptions,
    ) -> Option<Vec<String>> {
        let line_count = self.buffer_line_count(number)?;
        Some(self.gutter_lines_for(line_count, cursor_row, first, count, height, options))
    }

    fn gutter_lines_for(
        &self,
        line_count: usize,
        cursor_row: usize,
        first: usize,
        count: usize,
        height: usize,
        options: WindowOptions,
    ) -> Vec<String> {
        (first..first.saturating_add(count))
            .map(|row| self.gutter_line_for(row, line_count, cursor_row, height, options))
            .collect()
    }

    pub fn cursor_wcol(&self) -> usize {
        self.gutter_width().saturating_add(self.cursor_screen_col())
    }

    pub fn buffer_col_at_screen(&self, row: usize, screen: usize) -> usize {
        let tabstop = self.effective_indent().tabstop;
        let col = screen
            .saturating_sub(self.gutter_width())
            .saturating_add(self.viewport.leftcol());
        self.buffer.char_col(row, col, tabstop)
    }

    fn gutter_line_for(
        &self,
        row: usize,
        line_count: usize,
        cursor_row: usize,
        height: usize,
        options: WindowOptions,
    ) -> String {
        if !numbers_visible(options) {
            return String::new();
        }
        let width = number_column_width_for(options, line_count, height);
        match display_number(options, row, line_count, cursor_row) {
            Some((number, false)) => format!("{number:>width$} "),
            Some((number, true)) => format!("{number:<width$} "),
            None => " ".repeat(width.saturating_add(1)),
        }
    }
}

fn display_number(
    options: WindowOptions,
    row: usize,
    line_count: usize,
    cursor_row: usize,
) -> Option<(usize, bool)> {
    if row >= line_count {
        return None;
    }
    if options.number && !options.relativenumber {
        return Some((row.saturating_add(1), false));
    }
    let distance = row.abs_diff(cursor_row);
    if distance == 0 && options.number {
        return Some((row.saturating_add(1), true));
    }
    Some((distance, false))
}

fn numbers_visible(options: WindowOptions) -> bool {
    options.number || options.relativenumber
}

fn number_column_width_for(options: WindowOptions, line_count: usize, height: usize) -> usize {
    let reference = if options.relativenumber && !options.number {
        height
    } else {
        line_count
    };
    digit_count(reference).max(options.numberwidth.saturating_sub(1))
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
    fn gutter_shown_by_default() {
        let e = three_lines();
        assert_eq!(e.gutter_width(), 4);
        assert_eq!(e.gutter_lines(0, 3), vec!["  1 ", "  2 ", "  3 "]);
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
        e.set_number(false);
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
    fn signcolumn_shifts_cursor_and_mouse_columns() {
        let mut e = editor_with("abc\ndef");
        e.apply(Action::JumpTo(0, 0));
        e.set_signcolumn_width(1);
        assert_eq!(e.gutter_width(), 5);
        assert_eq!(e.cursor_wcol(), 5);
        assert_eq!(e.buffer_col_at_screen(0, 5), 0);
        e.set_number(false);
        e.set_relativenumber(false);
        assert_eq!(e.gutter_width(), 1);
        assert_eq!(e.buffer_col_at_screen(0, 1), 0);
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
        assert_eq!(e.cursor_wcol(), 11);
        e.set_number(false);
        assert_eq!(e.cursor_wcol(), 7);
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
