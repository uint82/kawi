use super::{Editor, WindowOptions};
use crate::folds::FoldState;

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
        self.signcolumn_width
            .saturating_add(self.fold_column_width())
            .saturating_add(numbers)
    }

    pub fn gutter_lines(&self, first: usize, count: usize) -> Vec<String> {
        let rows = self.visible_rows(first, count);
        let mut lines = self.gutter_lines_at(&rows);
        let blank = " ".repeat(self.gutter_width().saturating_sub(self.signcolumn_width));
        while lines.len() < count {
            lines.push(blank.clone());
        }
        lines
    }

    pub fn gutter_lines_at(&self, rows: &[usize]) -> Vec<String> {
        self.gutter_lines_for_rows(
            rows,
            self.buffer.line_count(),
            self.cursor.row(),
            self.view_height(),
            self.window_options(),
            &self.folds,
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
        let buffer = self.buffer_ref(number)?;
        let folds = self.fold_state_for(number)?;
        let rows = folds.visible_rows(first, count, buffer.line_count());
        Some(self.gutter_lines_for_rows(
            &rows,
            buffer.line_count(),
            cursor_row,
            height,
            options,
            folds,
        ))
    }

    pub fn buffer_gutter_lines_for_rows(
        &self,
        number: usize,
        rows: &[usize],
        cursor_row: usize,
        height: usize,
        options: WindowOptions,
    ) -> Option<Vec<String>> {
        let buffer = self.buffer_ref(number)?;
        let folds = self.fold_state_for(number)?;
        Some(self.gutter_lines_for_rows(
            rows,
            buffer.line_count(),
            cursor_row,
            height,
            options,
            folds,
        ))
    }

    fn gutter_lines_for_rows(
        &self,
        rows: &[usize],
        line_count: usize,
        cursor_row: usize,
        height: usize,
        options: WindowOptions,
        folds: &FoldState,
    ) -> Vec<String> {
        rows.iter()
            .map(|row| {
                let marker = if folds.column_width() == 0 {
                    ' '
                } else {
                    folds.marker(*row)
                };
                compose_gutter_line(
                    marker,
                    self.gutter_line_for(*row, line_count, cursor_row, height, options),
                    folds.column_width(),
                )
            })
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

fn compose_gutter_line(marker: char, numbers: String, fold_column: usize) -> String {
    if fold_column == 0 {
        return numbers;
    }
    format!("{marker}{numbers}")
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
mod tests;
