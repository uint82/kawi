use std::io::Write;

use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Attribute, SetAttribute, SetBackgroundColor, SetForegroundColor, SetUnderlineColor},
    terminal::{Clear, ClearType},
};
use kawi_buffer::unicode;

use crate::frame::DiagnosticUnderline;

use super::{saturate_u16, Cell, Style, Surface};

const AMBIGUOUS_CELL_WIDTH: usize = 2;

pub fn present<W: Write>(surface: &Surface, out: &mut W) -> Result<()> {
    for y in 0..surface.height() {
        write_row(out, surface, y)?;
    }
    Ok(())
}

pub(super) fn write_row<W: Write>(out: &mut W, surface: &Surface, y: usize) -> Result<()> {
    let base = surface.base_style();
    let mut writer = RowWriter::new(out);
    writer.move_to(0, y)?;
    writer.set_style(base)?;
    writer.clear_line()?;
    let Some(cells) = surface.row(y) else {
        return writer.finish();
    };
    let Some(start) = cells.iter().position(|cell| is_meaningful(cell, base)) else {
        return writer.finish();
    };
    let end = cells
        .iter()
        .rposition(|cell| is_meaningful(cell, base))
        .map_or(start.saturating_add(1), |index| index.saturating_add(1));
    writer.move_to(start, y)?;
    for (x, cell) in cells.iter().enumerate().take(end).skip(start) {
        if cell.is_continuation() {
            continue;
        }
        writer.write_cell(x, y, cell)?;
    }
    writer.finish()
}

fn is_meaningful(cell: &Cell, base: Style) -> bool {
    if cell.is_continuation() {
        return false;
    }
    cell.text() != " " || cell.width() != 1 || cell.style() != base
}

pub(super) struct RowWriter<'a, W> {
    out: &'a mut W,
    style: Option<Style>,
    run: String,
    force_move: bool,
}

impl<'a, W: Write> RowWriter<'a, W> {
    pub(super) fn new(out: &'a mut W) -> Self {
        Self {
            out,
            style: None,
            run: String::new(),
            force_move: false,
        }
    }

    pub(super) fn move_to(&mut self, x: usize, y: usize) -> Result<()> {
        self.flush()?;
        move_to(self.out, x, y)?;
        self.force_move = false;
        Ok(())
    }

    pub(super) fn set_style(&mut self, style: Style) -> Result<()> {
        if self.style == Some(style) {
            return Ok(());
        }
        self.flush()?;
        apply_style(self.out, style)?;
        self.style = Some(style);
        Ok(())
    }

    pub(super) fn clear_line(&mut self) -> Result<()> {
        self.flush()?;
        execute!(self.out, Clear(ClearType::CurrentLine)).context("clear surface row")?;
        Ok(())
    }

    pub(super) fn write_cell(&mut self, x: usize, y: usize, cell: &Cell) -> Result<()> {
        if cell.is_continuation() {
            return Ok(());
        }
        let style_changed = self.style != Some(cell.style());
        if style_changed || self.force_move {
            self.flush()?;
            if self.force_move {
                move_to(self.out, x, y)?;
                self.force_move = false;
            }
            if style_changed {
                apply_style(self.out, cell.style())?;
                self.style = Some(cell.style());
            }
        }
        if unicode::needs_terminal_reposition(cell.text()) {
            self.start_ambiguous_cell(x, y, cell.width())?;
        }
        self.run.push_str(cell.text());
        Ok(())
    }

    pub(super) fn finish(&mut self) -> Result<()> {
        self.flush()
    }

    fn start_ambiguous_cell(&mut self, x: usize, y: usize, width: usize) -> Result<()> {
        self.flush()?;
        move_to(self.out, x, y)?;
        if width > 1 {
            clear_ambiguous_cells(self.out)?;
            move_to(self.out, x, y)?;
        }
        self.force_move = true;
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        write_run(self.out, &self.run)?;
        self.run.clear();
        Ok(())
    }
}

fn clear_ambiguous_cells<W: Write>(out: &mut W) -> Result<()> {
    let spaces = [b' '; AMBIGUOUS_CELL_WIDTH];
    out.write_all(&spaces).context("clear ambiguous cell")
}

fn write_run<W: Write>(out: &mut W, run: &str) -> Result<()> {
    if run.is_empty() {
        return Ok(());
    }
    out.write_all(run.as_bytes()).context("write surface run")?;
    Ok(())
}

pub(super) fn move_to<W: Write>(out: &mut W, x: usize, y: usize) -> Result<()> {
    execute!(out, MoveTo(saturate_u16(x), saturate_u16(y))).context("move to surface cell")
}

fn apply_style<W: Write>(out: &mut W, style: Style) -> Result<()> {
    execute!(
        out,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(style.foreground()),
        SetBackgroundColor(style.background())
    )
    .context("apply surface colors")?;
    if style.is_bold() {
        execute!(out, SetAttribute(Attribute::Bold)).context("apply surface bold")?;
    }
    if style.is_dim() {
        execute!(out, SetAttribute(Attribute::Dim)).context("apply surface dim")?;
    }
    if style.is_italic() {
        execute!(out, SetAttribute(Attribute::Italic)).context("apply surface italic")?;
    }
    match style.underline() {
        Some(DiagnosticUnderline::Straight) => {
            execute!(out, SetAttribute(Attribute::Underlined))
                .context("apply surface underline")?;
        }
        Some(DiagnosticUnderline::Curly) => {
            execute!(out, SetAttribute(Attribute::Undercurled))
                .context("apply surface undercurl")?;
        }
        None => {}
    }
    if let Some(color) = style.underline_color() {
        execute!(out, SetUnderlineColor(color)).context("apply surface underline color")?;
    }
    if style.is_crossed_out() {
        execute!(out, SetAttribute(Attribute::CrossedOut))
            .context("apply surface strikethrough")?;
    }
    if style.is_hidden() {
        execute!(out, SetAttribute(Attribute::Hidden)).context("apply surface hidden")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crossterm::style::Color;

    use super::*;

    #[test]
    fn presents_wide_graphemes_once() {
        let mut surface = Surface::new(4, 1, Color::White, Color::Black);
        surface.write_text("a中");
        let mut out = Vec::new();
        present(&surface, &mut out).expect("present surface");
        let output = String::from_utf8(out).expect("utf8");
        assert_eq!(output.matches('中').count(), 1);
        assert!(output.contains("a中"));
    }

    #[test]
    fn presents_ambiguous_grapheme_before_absolute_positioning() {
        let mut surface = Surface::new(4, 1, Color::White, Color::Black);
        surface.write_text("\u{2764}\u{fe0f}X");
        let mut out = Vec::new();
        present(&surface, &mut out).expect("present surface");
        let output = String::from_utf8(out).expect("utf8");
        assert_eq!(output.matches('\u{2764}').count(), 1);
        assert!(output.contains("\u{1b}[1;3H"));
    }

    #[test]
    fn presents_soft_hyphen_before_absolute_positioning() {
        let mut surface = Surface::new(16, 1, Color::White, Color::Black);
        surface.write_text("co\u{ad}operate");
        let mut out = Vec::new();
        present(&surface, &mut out).expect("present surface");
        let output = String::from_utf8(out).expect("utf8");
        assert_eq!(output.matches('\u{ad}').count(), 1);
        assert!(output.contains("\u{1b}[1;4H"));
    }

    #[test]
    fn presents_single_cell_ambiguous_grapheme_before_absolute_positioning() {
        let mut surface = Surface::new(4, 1, Color::White, Color::Black);
        surface.write_text("1\u{fe0f}\u{20e3}X");
        let mut out = Vec::new();
        present(&surface, &mut out).expect("present surface");
        let output = String::from_utf8(out).expect("utf8");
        assert!(output.contains("\u{1b}[1;2H"));
    }
}
