use std::num::NonZeroU16;

const MIN_SIZE: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenAttrs {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenCell<'a> {
    pub text: &'a str,
    pub foreground: ScreenColor,
    pub background: ScreenColor,
    pub attrs: ScreenAttrs,
    pub wide: bool,
    pub wide_continuation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenCursorStyle {
    Block,
    Bar,
    Underline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenCursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
    pub style: ScreenCursorStyle,
}

pub struct TerminalScreen {
    parser: vt100::Parser,
    rows: u16,
    cols: u16,
}

impl TerminalScreen {
    pub fn new(rows: u16, cols: u16, scrollback: usize) -> Self {
        let rows = rows.max(MIN_SIZE);
        let cols = cols.max(MIN_SIZE);
        Self {
            parser: vt100::Parser::new(non_zero(rows), non_zero(cols), scrollback),
            rows,
            cols,
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let rows = rows.max(MIN_SIZE);
        let cols = cols.max(MIN_SIZE);
        if rows == self.rows && cols == self.cols {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        self.parser.set_size(non_zero(rows), non_zero(cols));
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn cell(&self, row: u16, col: u16) -> Option<ScreenCell<'_>> {
        let cell = self.parser.screen().cell(row, col)?;
        Some(ScreenCell {
            text: cell.contents(),
            foreground: screen_color(cell.fgcolor()),
            background: screen_color(cell.bgcolor()),
            attrs: ScreenAttrs {
                bold: cell.bold(),
                dim: cell.dim(),
                italic: cell.italic(),
                underline: cell.underline(),
                inverse: cell.inverse(),
            },
            wide: cell.is_wide(),
            wide_continuation: cell.is_wide_continuation(),
        })
    }

    pub fn cursor(&self) -> ScreenCursor {
        let (row, col) = self.parser.screen().cursor_position();
        ScreenCursor {
            row,
            col,
            visible: !self.parser.screen().hide_cursor(),
            style: cursor_style(self.parser.screen().cursor_style()),
        }
    }

    pub fn alternate_screen(&self) -> bool {
        self.parser.screen().alternate_screen()
    }

    pub fn application_cursor(&self) -> bool {
        self.parser.screen().application_cursor()
    }

    pub fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }

    pub fn scrollback(&self) -> usize {
        self.parser.screen().scrollback()
    }

    pub fn scrollback_len(&self) -> usize {
        self.parser.screen().scrollback_len()
    }

    pub fn set_scrollback(&mut self, rows: usize) {
        self.parser.screen_mut().set_scrollback(rows);
    }

    pub fn contents_between(
        &self,
        start_row: u16,
        start_col: u16,
        end_row: u16,
        end_col: u16,
    ) -> String {
        self.parser
            .screen()
            .contents_between(start_row, start_col, end_row, end_col)
    }
}

fn non_zero(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).unwrap_or(NonZeroU16::MIN)
}

fn screen_color(color: vt100::Color) -> ScreenColor {
    match color {
        vt100::Color::Default => ScreenColor::Default,
        vt100::Color::Idx(index) => ScreenColor::Indexed(index),
        vt100::Color::Rgb(red, green, blue) => ScreenColor::Rgb(red, green, blue),
    }
}

fn cursor_style(style: vt100::CursorStyle) -> ScreenCursorStyle {
    match style {
        vt100::CursorStyle::BlinkBlock | vt100::CursorStyle::SteadyBlock => {
            ScreenCursorStyle::Block
        }
        vt100::CursorStyle::BlinkUnderline | vt100::CursorStyle::SteadyUnderline => {
            ScreenCursorStyle::Underline
        }
        vt100::CursorStyle::BlinkVerticalBar | vt100::CursorStyle::SteadyVerticalBar => {
            ScreenCursorStyle::Bar
        }
        vt100::CursorStyle::Default => ScreenCursorStyle::Block,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processes_text_and_colors() {
        let mut screen = TerminalScreen::new(4, 20, 10);
        screen.process(b"hello \x1b[31mred\x1b[m");
        assert_eq!(screen.cell(0, 0).unwrap().text, "h");
        assert_eq!(screen.cell(0, 6).unwrap().text, "r");
        assert_eq!(
            screen.cell(0, 6).unwrap().foreground,
            ScreenColor::Indexed(1)
        );
    }

    #[test]
    fn resizes_to_the_requested_size() {
        let mut screen = TerminalScreen::new(4, 20, 10);
        screen.resize(10, 40);
        assert_eq!(screen.rows(), 10);
        assert_eq!(screen.cols(), 40);
        assert_eq!(screen.cursor().row, 0);
    }

    #[test]
    fn switches_to_the_alternate_screen() {
        let mut screen = TerminalScreen::new(4, 20, 10);
        screen.process(b"\x1b[?1049h");
        assert!(screen.alternate_screen());
        screen.process(b"\x1b[?1049l");
        assert!(!screen.alternate_screen());
    }

    #[test]
    fn tracks_application_cursor_and_bracketed_paste() {
        let mut screen = TerminalScreen::new(4, 20, 10);
        assert!(!screen.application_cursor());
        assert!(!screen.bracketed_paste());
        screen.process(b"\x1b[?1h\x1b[?2004h");
        assert!(screen.application_cursor());
        assert!(screen.bracketed_paste());
    }

    #[test]
    fn keeps_scrollback() {
        let mut screen = TerminalScreen::new(2, 10, 10);
        screen.process(b"one\r\ntwo\r\nthree\r\nfour");
        assert!(screen.scrollback_len() > 0);
        screen.set_scrollback(screen.scrollback_len());
        assert!(screen.scrollback() > 0);
        screen.set_scrollback(0);
        assert_eq!(screen.scrollback(), 0);
    }

    #[test]
    fn reports_cursor_visibility_and_style() {
        let mut screen = TerminalScreen::new(4, 20, 10);
        screen.process(b"\x1b[?25l\x1b[6 q");
        let cursor = screen.cursor();
        assert!(!cursor.visible);
        assert_eq!(cursor.style, ScreenCursorStyle::Bar);
        screen.process(b"\x1b[?25h\x1b[2 q");
        assert!(screen.cursor().visible);
        assert_eq!(screen.cursor().style, ScreenCursorStyle::Block);
    }

    #[test]
    fn extracts_selected_contents() {
        let mut screen = TerminalScreen::new(4, 20, 10);
        screen.process(b"alpha\r\nbeta");
        assert_eq!(screen.contents_between(0, 0, 1, 4), "alpha\nbeta");
    }

    #[test]
    fn zero_sizes_are_clamped() {
        let screen = TerminalScreen::new(0, 0, 10);
        assert_eq!(screen.rows(), 1);
        assert_eq!(screen.cols(), 1);
    }
}
