use anyhow::{Context, Result};
use crossterm::style::Color;
use kawi_completion::CompletionKind;
use std::io::Write;
use std::ops::Range;

use crate::frame::CompletionLine;
use crate::paint;
use crate::surface::Surface;
use crate::text::{display_width, truncate_to_display_width};
use crate::theme::Theme;
use kawi_buffer::unicode;

use super::furniture::write_spaces;

const DETAIL_GAP: usize = 2;
const KIND_GAP: usize = 2;

pub(super) fn draw_row(
    out: &mut Surface,
    theme: &Theme,
    line: &CompletionLine,
    width: usize,
    kind_width: usize,
    selected: bool,
) -> Result<()> {
    apply_row_colors(out, theme, selected);
    out.set_crossed_out(line.deprecated);
    let column_width = width.saturating_sub(kind_space(kind_width));
    let used = write_primary(out, theme, line, column_width, selected)?;
    out.set_foreground(label_color(theme, selected, false));
    write_spaces(out, column_width.saturating_sub(used))?;
    out.set_crossed_out(false);
    write_kind(out, theme, line.kind, kind_width, selected)?;
    paint::restore_base(out, theme);
    Ok(())
}

pub(super) fn draw_blank(
    out: &mut Surface,
    theme: &Theme,
    width: usize,
    selected: bool,
) -> Result<()> {
    apply_row_colors(out, theme, selected);
    out.set_crossed_out(false);
    write_spaces(out, width)?;
    paint::restore_base(out, theme);
    Ok(())
}

fn apply_row_colors(out: &mut Surface, theme: &Theme, selected: bool) {
    if selected {
        paint::apply_match(out, &theme.selection);
    } else {
        paint::apply_colors(out, theme.base.foreground, theme.float_background);
    }
}

fn write_primary(
    out: &mut Surface,
    theme: &Theme,
    line: &CompletionLine,
    width: usize,
    selected: bool,
) -> Result<usize> {
    let used = write_label(out, theme, line, width, selected)?;
    let Some(detail) = line.detail.as_deref() else {
        return Ok(used);
    };
    let remaining = width.saturating_sub(used);
    if remaining <= DETAIL_GAP {
        return Ok(used);
    }
    out.set_foreground(detail_color(theme, selected));
    let detail = truncate_to_display_width(detail, remaining - DETAIL_GAP);
    write!(out, "  {detail}").context("write completion detail")?;
    Ok(used + DETAIL_GAP + display_width(detail))
}

fn write_label(
    out: &mut Surface,
    theme: &Theme,
    line: &CompletionLine,
    column_width: usize,
    selected: bool,
) -> Result<usize> {
    let mut used: usize = 0;
    let mut highlight_index = 0;
    for grapheme in unicode::graphemes(&line.label) {
        let start = grapheme.start_byte();
        let text = grapheme.text();
        let width = display_width(text);
        if used.saturating_add(width) > column_width {
            break;
        }
        let highlighted = is_highlighted(&line.highlights, &mut highlight_index, start);
        out.set_foreground(label_color(theme, selected, highlighted));
        write!(out, "{text}").context("write completion label")?;
        used = used.saturating_add(width);
    }
    Ok(used)
}

fn is_highlighted(highlights: &[Range<usize>], cursor: &mut usize, start: usize) -> bool {
    while highlights
        .get(*cursor)
        .is_some_and(|range| range.end <= start)
    {
        *cursor += 1;
    }
    highlights
        .get(*cursor)
        .is_some_and(|range| range.start <= start && start < range.end)
}

fn write_kind(
    out: &mut Surface,
    theme: &Theme,
    kind: Option<CompletionKind>,
    kind_width: usize,
    selected: bool,
) -> Result<()> {
    if kind_width == 0 {
        return Ok(());
    }
    write_spaces(out, KIND_GAP)?;
    let Some(kind) = kind else {
        return write_spaces(out, kind_width);
    };
    let name = kind.name();
    write_spaces(out, kind_width.saturating_sub(display_width(name)))?;
    out.set_foreground(kind_color(theme, kind, selected));
    write!(out, "{name}").context("write completion kind")
}

fn kind_space(kind_width: usize) -> usize {
    if kind_width == 0 {
        0
    } else {
        KIND_GAP + kind_width
    }
}

fn label_color(theme: &Theme, selected: bool, highlighted: bool) -> Color {
    if selected {
        theme.selection.foreground
    } else if highlighted {
        theme.completion.matched()
    } else {
        theme.base.foreground
    }
}

fn detail_color(theme: &Theme, selected: bool) -> Color {
    if selected {
        theme.selection.foreground
    } else {
        theme.non_text
    }
}

fn kind_color(theme: &Theme, kind: CompletionKind, selected: bool) -> Color {
    if selected {
        theme.selection.foreground
    } else {
        theme.completion.color(kind)
    }
}
