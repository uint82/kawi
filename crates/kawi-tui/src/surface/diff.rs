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
mod tests {
    use crossterm::{
        execute,
        style::{Color, SetForegroundColor},
    };

    use super::*;

    fn surface(width: usize, height: usize) -> Surface {
        Surface::new(width, height, Color::White, Color::Black)
    }

    fn diff_output(previous: &Surface, next: &Surface) -> String {
        let mut out = Vec::new();
        diff(previous, next, &mut out).expect("diff surface");
        String::from_utf8(out).expect("utf8")
    }

    #[test]
    fn identical_surfaces_emit_nothing() {
        let previous = surface(4, 2);
        let next = surface(4, 2);
        assert!(diff_output(&previous, &next).is_empty());
    }

    #[test]
    fn changed_text_emits_only_the_changed_cell() {
        let mut previous = surface(4, 1);
        previous.write_text("abc");
        let mut next = surface(4, 1);
        next.write_text("axc");
        let output = diff_output(&previous, &next);
        assert!(output.contains('x'));
        assert!(!output.contains('a'));
        assert!(!output.contains('c'));
    }

    #[test]
    fn style_only_changes_rewrite_the_cell() {
        let mut previous = surface(2, 1);
        previous.write_text("a");
        let mut next = surface(2, 1);
        next.set_foreground(Color::Red);
        next.write_text("a");
        let output = diff_output(&previous, &next);
        let mut expected = Vec::new();
        execute!(expected, SetForegroundColor(Color::Red)).expect("foreground sequence");
        assert!(output.contains('a'));
        assert!(output.contains(&String::from_utf8(expected).expect("utf8")));
    }

    #[test]
    fn replacing_a_wide_grapheme_rewrites_both_cells() {
        let mut previous = surface(4, 1);
        previous.write_text("中");
        let mut next = surface(4, 1);
        next.write_text("x");
        let output = diff_output(&previous, &next);
        assert_eq!(output.matches('x').count(), 1);
        assert!(!output.contains('中'));
    }

    #[test]
    fn ambiguous_grapheme_repositions_following_cell() {
        let previous = surface(5, 1);
        let mut next = surface(5, 1);
        next.write_text("\u{2764}\u{fe0f}X");
        let output = diff_output(&previous, &next);
        assert_eq!(output.matches('\u{2764}').count(), 1);
        assert!(output.contains("\u{1b}[1;3H"));
    }

    #[test]
    fn unchanged_rows_emit_nothing() {
        let mut previous = surface(2, 2);
        previous.move_to(0, 1);
        previous.write_text("a");
        let mut next = surface(2, 2);
        next.move_to(0, 1);
        next.write_text("b");
        let output = diff_output(&previous, &next);
        assert!(!output.contains("\u{1b}[1;1H"));
        assert!(output.contains("\u{1b}[2;1H"));
    }

    #[test]
    fn size_changes_fall_back_to_a_full_present() {
        let mut previous = surface(2, 1);
        previous.write_text("ab");
        let mut next = surface(3, 1);
        next.write_text("abc");
        assert!(diff_output(&previous, &next).contains("abc"));
    }
}
