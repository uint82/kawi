use kawi_terminal::{ScreenAttrs, ScreenColor, ScreenCursorStyle, TerminalScreen};
use kawi_tui::frame::{
    CursorStyle, TerminalAttrs, TerminalColor, TerminalCursor, TerminalLine, TerminalSpan,
};

use super::{TerminalMode, TerminalUi};

impl TerminalUi {
    pub fn lines(&self, width: usize, height: usize) -> Vec<TerminalLine> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        let screen = session.screen();
        let rows = height.min(usize::from(screen.rows()));
        let cols = width.min(usize::from(screen.cols()));
        (0..rows).map(|row| line_at(screen, row, cols)).collect()
    }

    pub fn cursor(&self) -> Option<TerminalCursor> {
        let session = self.session.as_ref()?;
        if self.mode == TerminalMode::Normal {
            return Some(TerminalCursor {
                row: usize::from(self.normal_cursor.0),
                col: usize::from(self.normal_cursor.1),
                style: CursorStyle::Block,
            });
        }
        let screen = session.screen();
        if screen.scrollback() > 0 {
            return None;
        }
        let cursor = screen.cursor();
        if !cursor.visible {
            return None;
        }
        Some(TerminalCursor {
            row: usize::from(cursor.row),
            col: usize::from(cursor.col),
            style: cursor_style(cursor.style),
        })
    }
}

pub(super) fn line_at(screen: &TerminalScreen, row: usize, cols: usize) -> TerminalLine {
    let mut spans = Vec::new();
    let mut current: Option<TerminalSpan> = None;
    let mut col = 0;
    while col < cols {
        let Some(cell) = screen.cell(row as u16, col as u16) else {
            col += 1;
            continue;
        };
        if cell.wide_continuation {
            col += 1;
            continue;
        }
        let text = if cell.text.is_empty() { " " } else { cell.text };
        let foreground = screen_color(cell.foreground);
        let background = screen_color(cell.background);
        let attrs = screen_attrs(cell.attrs);
        match current.as_mut() {
            Some(span)
                if span.foreground == foreground
                    && span.background == background
                    && span.attrs == attrs =>
            {
                span.text.push_str(text);
            }
            Some(_) => {
                spans.push(current.take().expect("current span exists"));
                current = Some(new_span(text, foreground, background, attrs));
            }
            None => current = Some(new_span(text, foreground, background, attrs)),
        }
        col += if cell.wide { 2 } else { 1 };
    }
    if let Some(span) = current {
        spans.push(span);
    }
    TerminalLine { spans }
}

fn new_span(
    text: &str,
    foreground: TerminalColor,
    background: TerminalColor,
    attrs: TerminalAttrs,
) -> TerminalSpan {
    TerminalSpan {
        text: text.to_string(),
        foreground,
        background,
        attrs,
    }
}

fn screen_color(color: ScreenColor) -> TerminalColor {
    match color {
        ScreenColor::Default => TerminalColor::Default,
        ScreenColor::Indexed(index) => TerminalColor::Indexed(index),
        ScreenColor::Rgb(red, green, blue) => TerminalColor::Rgb(red, green, blue),
    }
}

fn screen_attrs(attrs: ScreenAttrs) -> TerminalAttrs {
    TerminalAttrs {
        bold: attrs.bold,
        dim: attrs.dim,
        italic: attrs.italic,
        underline: attrs.underline,
        inverse: attrs.inverse,
    }
}

fn cursor_style(style: ScreenCursorStyle) -> CursorStyle {
    match style {
        ScreenCursorStyle::Block => CursorStyle::Block,
        ScreenCursorStyle::Bar => CursorStyle::Bar,
        ScreenCursorStyle::Underline => CursorStyle::Underline,
    }
}
