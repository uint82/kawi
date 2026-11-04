mod ansi;
mod cell;
mod style;

use std::io::{self, Write};

use crossterm::style::Color;
use unicode_segmentation::UnicodeSegmentation;

use crate::frame::DiagnosticUnderline;
use crate::text::{display_width, sanitize_char, TAB_WIDTH};

pub use ansi::present;
pub use cell::Cell;
pub use style::Style;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

pub struct Surface {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    cursor: Position,
    style: Style,
    base: Style,
}

impl Surface {
    pub fn new(width: usize, height: usize, foreground: Color, background: Color) -> Self {
        let base = Style::new(foreground, background);
        Self {
            width,
            height,
            cells: vec![Cell::blank(base); width.saturating_mul(height)],
            cursor: Position::default(),
            style: base,
            base,
        }
    }

    pub fn fit(&mut self, width: usize, height: usize, foreground: Color, background: Color) {
        if self.width != width || self.height != height {
            *self = Self::new(width, height, foreground, background);
            return;
        }
        self.begin(foreground, background);
    }

    pub fn begin(&mut self, foreground: Color, background: Color) {
        self.base = Style::new(foreground, background);
        self.style = self.base;
        self.cursor = Position::default();
        for cell in &mut self.cells {
            cell.reset(self.base);
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cell(&self, x: usize, y: usize) -> Option<&Cell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.cells
            .get(y.saturating_mul(self.width).saturating_add(x))
    }

    pub fn cursor(&self) -> Position {
        self.cursor
    }

    pub fn set_foreground(&mut self, color: Color) {
        self.style.set_foreground(color);
    }

    pub fn set_background(&mut self, color: Color) {
        self.style.set_background(color);
    }

    pub fn set_colors(&mut self, foreground: Color, background: Color) {
        self.style.set_colors(foreground, background);
    }

    pub fn set_bold(&mut self, enabled: bool) {
        self.style.set_bold(enabled);
    }

    pub fn set_dim(&mut self, enabled: bool) {
        self.style.set_dim(enabled);
    }

    pub fn set_italic(&mut self, enabled: bool) {
        self.style.set_italic(enabled);
    }

    pub fn set_crossed_out(&mut self, enabled: bool) {
        self.style.set_crossed_out(enabled);
    }

    pub fn set_hidden(&mut self, enabled: bool) {
        self.style.set_hidden(enabled);
    }

    pub fn set_underline(&mut self, style: DiagnosticUnderline, color: Color) {
        self.style.set_underline(style, color);
    }

    pub fn set_plain_underline(&mut self) {
        self.style.set_plain_underline();
    }

    pub fn reset_attributes(&mut self) {
        self.style.reset_attributes();
    }

    pub fn move_to(&mut self, x: usize, y: usize) {
        self.cursor = Position { x, y };
    }

    pub fn clear_line(&mut self) {
        self.clear_row(self.cursor.y);
    }

    pub fn clear_tail(&mut self) {
        let y = self.cursor.y;
        if y >= self.height {
            return;
        }
        let x = self.continuation_start(self.cursor.x, y);
        for column in x..self.width {
            self.set_blank(column, y);
        }
    }

    pub fn clear_all(&mut self) {
        for y in 0..self.height {
            self.clear_row(y);
        }
    }

    pub fn write_text(&mut self, text: &str) {
        for grapheme in text.graphemes(true) {
            self.write_grapheme(grapheme);
        }
    }

    fn write_grapheme(&mut self, grapheme: &str) {
        if grapheme == "\t" {
            let spaces = TAB_WIDTH.saturating_sub(self.cursor.x % TAB_WIDTH);
            self.write_spaces(spaces);
            return;
        }
        let sanitized: String;
        let grapheme = if grapheme.chars().any(char::is_control) {
            sanitized = grapheme.chars().map(sanitize_char).collect();
            sanitized.as_str()
        } else {
            grapheme
        };
        let width = display_width(grapheme);
        if width == 0 {
            self.append_combining(grapheme);
            return;
        }
        self.write_glyph(grapheme, width);
    }

    fn write_spaces(&mut self, count: usize) {
        for _ in 0..count {
            let x = self.cursor.x;
            let y = self.cursor.y;
            if x >= self.width || y >= self.height {
                return;
            }
            self.clear_glyph_at(x, y);
            self.set_blank(x, y);
            self.cursor.x = x.saturating_add(1);
        }
    }

    fn write_glyph(&mut self, text: &str, width: usize) {
        let x = self.cursor.x;
        let y = self.cursor.y;
        if y >= self.height || x.saturating_add(width) > self.width {
            self.cursor.x = self.width;
            return;
        }
        self.clear_glyph_at(x, y);
        if width > 1 {
            self.clear_glyph_at(x.saturating_add(1), y);
        }
        let style = self.style;
        if let Some(cell) = self.cell_mut(x, y) {
            cell.set(text, u8::try_from(width).unwrap_or(1), style);
        }
        for offset in 1..width {
            if let Some(cell) = self.cell_mut(x.saturating_add(offset), y) {
                cell.set("", 0, style);
            }
        }
        self.cursor.x = x.saturating_add(width);
    }

    fn append_combining(&mut self, text: &str) {
        let y = self.cursor.y;
        if self.cursor.x == 0 || y >= self.height {
            return;
        }
        let mut x = self.cursor.x.saturating_sub(1);
        if self.cell(x, y).is_some_and(Cell::is_continuation) {
            x = x.saturating_sub(1);
        }
        if let Some(cell) = self.cell_mut(x, y) {
            cell.append(text);
        }
    }

    fn continuation_start(&self, x: usize, y: usize) -> usize {
        if self.cell(x, y).is_some_and(Cell::is_continuation) {
            x.saturating_sub(1)
        } else {
            x
        }
    }

    fn clear_glyph_at(&mut self, x: usize, y: usize) {
        if x >= self.width || y >= self.height {
            return;
        }
        let start = self.continuation_start(x, y);
        let width = self.cell(start, y).map_or(1, |cell| cell.width().max(1));
        for column in start..start.saturating_add(width).min(self.width) {
            self.set_blank(column, y);
        }
    }

    fn clear_row(&mut self, y: usize) {
        if y >= self.height {
            return;
        }
        for x in 0..self.width {
            self.set_blank(x, y);
        }
    }

    fn set_blank(&mut self, x: usize, y: usize) {
        let style = self.style;
        if let Some(cell) = self.cell_mut(x, y) {
            cell.reset(style);
        }
    }

    fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = y.saturating_mul(self.width).saturating_add(x);
        self.cells.get_mut(index)
    }
}

impl Write for Surface {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let text = std::str::from_utf8(buffer)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.write_text(text);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
