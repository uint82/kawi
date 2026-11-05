use std::io::Write;

use anyhow::Result;

use super::{ansi, Surface};

pub fn diff<W: Write>(previous: &Surface, next: &Surface, out: &mut W) -> Result<()> {
    if previous.width() != next.width() || previous.height() != next.height() {
        return ansi::present(next, out);
    }
    for y in 0..next.height() {
        diff_row(previous, next, y, out)?;
    }
    Ok(())
}

fn diff_row<W: Write>(previous: &Surface, next: &Surface, y: usize, out: &mut W) -> Result<()> {
    if previous.row(y) == next.row(y) {
        return Ok(());
    }
    if previous.row_is_width_stable(y) && next.row_is_width_stable(y) {
        return diff_stable_row(previous, next, y, out);
    }
    ansi::write_row(out, next, y)
}

fn diff_stable_row<W: Write>(
    previous: &Surface,
    next: &Surface,
    y: usize,
    out: &mut W,
) -> Result<()> {
    let mut writer = ansi::RowWriter::new(out);
    let mut x = 0;
    while x < next.width() {
        let Some((start, end)) = changed_glyph(previous, next, x, y) else {
            x = x.saturating_add(1);
            continue;
        };
        let end = extend_run(previous, next, start, end, y);
        write_range(&mut writer, next, start, end, y)?;
        x = end;
    }
    writer.finish()
}

fn extend_run(previous: &Surface, next: &Surface, start: usize, mut end: usize, y: usize) -> usize {
    while end < next.width() {
        let Some((glyph_start, glyph_end)) = changed_glyph(previous, next, end, y) else {
            break;
        };
        if glyph_start != end {
            break;
        }
        end = glyph_end;
    }
    end.max(start.saturating_add(1)).min(next.width())
}

fn write_range<W: Write>(
    writer: &mut ansi::RowWriter<'_, W>,
    surface: &Surface,
    start: usize,
    end: usize,
    y: usize,
) -> Result<()> {
    let Some(mut x) = next_glyph(surface, start, end, y) else {
        return Ok(());
    };
    writer.move_to(x, y)?;
    while x < end {
        let Some(cell) = surface.cell(x, y) else {
            break;
        };
        writer.write_cell(x, y, cell)?;
        x = x.saturating_add(cell.width().max(1));
        let Some(next) = next_glyph(surface, x, end, y) else {
            break;
        };
        if next != x {
            writer.move_to(next, y)?;
        }
        x = next;
    }
    Ok(())
}

fn next_glyph(surface: &Surface, mut x: usize, end: usize, y: usize) -> Option<usize> {
    while x < end {
        let cell = surface.cell(x, y)?;
        if !cell.is_continuation() {
            return Some(x);
        }
        x = x.saturating_add(1);
    }
    None
}

fn changed_glyph(previous: &Surface, next: &Surface, x: usize, y: usize) -> Option<(usize, usize)> {
    let previous_cell = previous.cell(x, y)?;
    let next_cell = next.cell(x, y)?;
    if previous_cell == next_cell {
        return None;
    }
    let (previous_start, previous_end) = previous.glyph_span(x, y);
    let (next_start, next_end) = next.glyph_span(x, y);
    Some((previous_start.min(next_start), previous_end.max(next_end)))
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
