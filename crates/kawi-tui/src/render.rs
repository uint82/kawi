use anyhow::{Context, Result};
use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    execute,
    style::{Attribute, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{Stdout, Write};

use crate::frame::{Frame, SearchSpan};
use crate::statusbar::{draw_status, StatusInfo};
use crate::theme::{MatchColors, Theme};
use crate::viewport::Viewport;

pub fn draw(out: &mut Stdout, frame: &Frame) -> Result<()> {
    clear_all(out)?;
    draw_visible_lines(out, frame.lines, frame.viewport)?;
    draw_highlight(out, frame)?;
    draw_status_row(out, frame.status, frame.viewport)?;
    draw_message_row(out, frame)?;
    apply_cursor_style(out, frame.insert_mode)?;
    place_cursor(out, frame)?;
    flush(out)?;
    Ok(())
}

fn clear_all(out: &mut Stdout) -> Result<()> {
    execute!(out, Clear(ClearType::All)).context("clear screen")?;
    Ok(())
}

fn draw_visible_lines(out: &mut Stdout, lines: &[String], viewport: &Viewport) -> Result<()> {
    for (screen_row, line) in lines.iter().enumerate().take(viewport.height()) {
        draw_one_line(out, screen_row, line, viewport.width())?;
    }
    Ok(())
}

fn draw_one_line(out: &mut Stdout, screen_row: usize, text: &str, width: usize) -> Result<()> {
    execute!(out, MoveTo(0, saturate_u16(screen_row))).context("move to line start")?;
    write!(out, "{}", truncate_to_width(text, width)).context("write line")?;
    Ok(())
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

pub struct HighlightSpan {
    pub screen_row: usize,
    pub from: usize,
    pub text: String,
    pub tint: Tint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Selection,
    SearchMatch,
    SearchCurrent,
}

pub fn highlight_spans(
    window: &[String],
    top: usize,
    selection: Option<((usize, usize), (usize, usize))>,
    viewport: &Viewport,
) -> Vec<HighlightSpan> {
    let Some(((start_row, start_col), (end_row, end_col))) = selection else {
        return Vec::new();
    };
    if (start_row, start_col) == (end_row, end_col) {
        return Vec::new();
    }
    let mut spans = Vec::new();
    for screen_row in 0..window.len().min(viewport.height()) {
        collect_span(
            &mut spans,
            window,
            top,
            viewport,
            screen_row,
            ((start_row, start_col), (end_row, end_col)),
        );
    }
    spans
}

fn collect_span(
    spans: &mut Vec<HighlightSpan>,
    window: &[String],
    top: usize,
    viewport: &Viewport,
    screen_row: usize,
    range: ((usize, usize), (usize, usize)),
) {
    if let Some(span) = selected_segment(window, top, viewport, screen_row, range) {
        spans.push(span);
    }
}

fn selected_segment(
    window: &[String],
    top: usize,
    viewport: &Viewport,
    screen_row: usize,
    range: ((usize, usize), (usize, usize)),
) -> Option<HighlightSpan> {
    let ((start_row, start_col), (end_row, end_col)) = range;
    let buffer_row = top.saturating_add(screen_row);
    if buffer_row < start_row || buffer_row > end_row {
        return None;
    }
    let text = window.get(screen_row)?;
    let width = viewport.width();
    let from = segment_start(buffer_row, start_row, start_col).min(width);
    let to = segment_end(buffer_row, text, end_row, end_col).min(width);
    if from >= to {
        return None;
    }
    Some(HighlightSpan {
        screen_row,
        from,
        text: slice_cols(text, from, to),
        tint: Tint::Selection,
    })
}

pub fn search_highlight_spans(
    spans: &[SearchSpan],
    window: &[String],
    top: usize,
    viewport: &Viewport,
) -> Vec<HighlightSpan> {
    spans
        .iter()
        .filter_map(|span| search_segment(span, window, top, viewport))
        .collect()
}

fn search_segment(
    span: &SearchSpan,
    window: &[String],
    top: usize,
    viewport: &Viewport,
) -> Option<HighlightSpan> {
    if span.row < top {
        return None;
    }
    let screen_row = span.row.saturating_sub(top);
    let text = window.get(screen_row)?;
    if screen_row >= viewport.height() {
        return None;
    }
    let width = viewport.width();
    let from = span.start.min(width);
    let to = span.end.min(width);
    if from >= to {
        return None;
    }
    Some(HighlightSpan {
        screen_row,
        from,
        text: slice_cols(text, from, to),
        tint: search_tint(span.current),
    })
}

fn search_tint(current: bool) -> Tint {
    if current {
        return Tint::SearchCurrent;
    }
    Tint::SearchMatch
}

fn draw_highlight(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let theme = Theme::default();
    for span in search_highlight_spans(
        frame.search_spans,
        frame.lines,
        frame.viewport.top(),
        frame.viewport,
    ) {
        paint_span(out, &span, &theme)?;
    }
    for span in highlight_spans(
        frame.lines,
        frame.viewport.top(),
        frame.selection,
        frame.viewport,
    ) {
        paint_span(out, &span, &theme)?;
    }
    Ok(())
}

fn paint_span(out: &mut Stdout, span: &HighlightSpan, theme: &Theme) -> Result<()> {
    move_to_segment(out, span.from, span.screen_row)?;
    match span.tint {
        Tint::Selection => paint_reversed(out, &span.text),
        Tint::SearchMatch => paint_tinted(out, &span.text, &theme.search_match),
        Tint::SearchCurrent => paint_tinted(out, &span.text, &theme.search_current),
    }
}

fn paint_reversed(out: &mut Stdout, text: &str) -> Result<()> {
    enable_reverse(out)?;
    write_segment(out, text)?;
    disable_reverse(out)?;
    Ok(())
}

fn paint_tinted(out: &mut Stdout, text: &str, colors: &MatchColors) -> Result<()> {
    execute!(
        out,
        SetForegroundColor(colors.foreground),
        SetBackgroundColor(colors.background)
    )
    .context("tint span")?;
    write_segment(out, text)?;
    execute!(out, ResetColor).context("reset tint")?;
    Ok(())
}

fn segment_start(row: usize, start_row: usize, start_col: usize) -> usize {
    if row == start_row {
        start_col
    } else {
        0
    }
}

fn segment_end(row: usize, text: &str, end_row: usize, end_col: usize) -> usize {
    if row == end_row {
        end_col.saturating_add(1)
    } else {
        char_len(text)
    }
}

fn slice_cols(text: &str, from: usize, to: usize) -> String {
    text.chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}

fn move_to_segment(out: &mut Stdout, col: usize, screen_row: usize) -> Result<()> {
    execute!(out, MoveTo(saturate_u16(col), saturate_u16(screen_row)))
        .context("move to segment")?;
    Ok(())
}

fn enable_reverse(out: &mut Stdout) -> Result<()> {
    execute!(out, SetAttribute(Attribute::Reverse)).context("enable reverse")?;
    Ok(())
}

fn write_segment(out: &mut Stdout, text: &str) -> Result<()> {
    write!(out, "{text}").context("write segment")?;
    Ok(())
}

fn disable_reverse(out: &mut Stdout) -> Result<()> {
    execute!(out, SetAttribute(Attribute::Reset)).context("disable reverse")?;
    Ok(())
}

fn draw_status_row(out: &mut Stdout, status: &StatusInfo, viewport: &Viewport) -> Result<()> {
    draw_status(out, status, viewport.width(), viewport.height())
}

fn draw_message_row(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let layout = layout_message_row(
        frame.command,
        frame.search,
        frame.message,
        frame.search_status,
        frame.showcmd,
        frame.viewport.width(),
    );
    move_to_message_row(out, frame.viewport)?;
    write!(out, "{}", layout.line).context("write message row")?;
    if let Some(column) = layout.showcmd_col {
        move_to_showcmd(out, column, frame.viewport)?;
        write!(out, "{}", layout.showcmd).context("write showcmd")?;
    }
    Ok(())
}

struct MessageLayout {
    line: String,
    showcmd: String,
    showcmd_col: Option<usize>,
}

fn layout_message_row(
    command: Option<&str>,
    search: Option<&str>,
    message: Option<&str>,
    search_status: Option<&str>,
    showcmd: Option<&str>,
    width: usize,
) -> MessageLayout {
    let show = showcmd.unwrap_or("");
    let left = message_left(command, search, message, search_status);
    MessageLayout {
        line: truncate_to_width(&left, left_width(width, show)),
        showcmd: truncate_to_width(show, width),
        showcmd_col: showcmd_column(show, width),
    }
}

fn message_left(
    command: Option<&str>,
    search: Option<&str>,
    message: Option<&str>,
    search_status: Option<&str>,
) -> String {
    if let Some(text) = command {
        return format!(":{text}");
    }
    if let Some(text) = search {
        return format!("/{text}");
    }
    message.or(search_status).unwrap_or("").to_string()
}

fn left_width(width: usize, showcmd: &str) -> usize {
    width.saturating_sub(char_len(showcmd))
}

fn showcmd_column(showcmd: &str, width: usize) -> Option<usize> {
    if showcmd.is_empty() {
        return None;
    }
    Some(width.saturating_sub(char_len(showcmd)))
}

fn move_to_showcmd(out: &mut Stdout, col: usize, viewport: &Viewport) -> Result<()> {
    execute!(
        out,
        MoveTo(
            saturate_u16(col),
            saturate_u16(viewport.height().saturating_add(1))
        )
    )
    .context("move to showcmd")?;
    Ok(())
}

fn move_to_message_row(out: &mut Stdout, viewport: &Viewport) -> Result<()> {
    execute!(
        out,
        MoveTo(0, saturate_u16(viewport.height().saturating_add(1)))
    )
    .context("move to message row")?;
    Ok(())
}

fn apply_cursor_style(out: &mut Stdout, insert_mode: bool) -> Result<()> {
    execute!(out, cursor_style(insert_mode)).context("set cursor style")?;
    Ok(())
}

fn cursor_style(insert_mode: bool) -> SetCursorStyle {
    if insert_mode {
        SetCursorStyle::SteadyBar
    } else {
        SetCursorStyle::SteadyBlock
    }
}

fn place_cursor(out: &mut Stdout, frame: &Frame) -> Result<()> {
    if let Some(col) = prompt_cursor_col(
        frame.command,
        frame.search,
        frame.viewport.width(),
        frame.showcmd,
    ) {
        return place_prompt_cursor(out, col, frame.viewport);
    }
    place_cursor_relative(out, frame.col, frame.row, frame.viewport)
}

fn place_cursor_relative(
    out: &mut Stdout,
    col: usize,
    row: usize,
    viewport: &Viewport,
) -> Result<()> {
    let (x, y) = screen_coords(col, row, viewport);
    execute!(out, MoveTo(x, y)).context("place cursor")?;
    Ok(())
}

fn place_prompt_cursor(out: &mut Stdout, col: usize, viewport: &Viewport) -> Result<()> {
    execute!(
        out,
        MoveTo(
            saturate_u16(col),
            saturate_u16(viewport.height().saturating_add(1))
        )
    )
    .context("place prompt cursor")?;
    Ok(())
}

pub fn prompt_cursor_col(
    command: Option<&str>,
    search: Option<&str>,
    width: usize,
    showcmd: Option<&str>,
) -> Option<usize> {
    let text = prompt_text(command, search)?;
    Some(prompt_col(text, width, showcmd))
}

fn prompt_text<'a>(command: Option<&'a str>, search: Option<&'a str>) -> Option<&'a str> {
    match (command, search) {
        (Some(text), _) => Some(text),
        (None, Some(text)) => Some(text),
        (None, None) => None,
    }
}

fn prompt_col(text: &str, width: usize, showcmd: Option<&str>) -> usize {
    prompt_home()
        .saturating_add(char_len(text))
        .min(prompt_edge(width, showcmd))
}

fn prompt_home() -> usize {
    1
}

fn prompt_edge(width: usize, showcmd: Option<&str>) -> usize {
    width
        .saturating_sub(char_len(showcmd.unwrap_or("")))
        .saturating_sub(1)
}

fn screen_coords(col: usize, row: usize, viewport: &Viewport) -> (u16, u16) {
    let x = saturate_u16(col.min(viewport.width().saturating_sub(1)));
    let y = saturate_u16(viewport.screen_row(row));
    (x, y)
}

fn flush(out: &mut Stdout) -> Result<()> {
    out.flush().context("flush stdout")?;
    Ok(())
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn saturate_u16(v: usize) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Viewport {
        Viewport::new(10, 20)
    }

    fn lines_of(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn no_selection_has_no_spans() {
        let lines = lines_of("abcdef");
        assert!(highlight_spans(&lines, 0, None, &viewport()).is_empty());
    }

    #[test]
    fn point_selection_has_no_spans() {
        let lines = lines_of("abcdef");
        let spans = highlight_spans(&lines, 0, Some(((0, 2), (0, 2))), &viewport());
        assert!(spans.is_empty());
    }

    #[test]
    fn range_selection_spans_chars() {
        let lines = lines_of("abcdef");
        let spans = highlight_spans(&lines, 0, Some(((0, 1), (0, 3))), &viewport());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].screen_row, 0);
        assert_eq!(spans[0].from, 1);
        assert_eq!(spans[0].text, "bcd");
    }

    #[test]
    fn multiline_selection_spans_rows() {
        let lines = lines_of("ab\ncd\nef");
        let spans = highlight_spans(&lines, 0, Some(((0, 1), (2, 1))), &viewport());
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "b");
        assert_eq!(spans[1].text, "cd");
        assert_eq!(spans[2].text, "ef");
    }

    #[test]
    fn layout_prefers_command_over_message() {
        let layout = layout_message_row(Some("w"), None, Some("saved"), None, None, 20);
        assert_eq!(layout.line, ":w");
        assert_eq!(layout.showcmd_col, None);
    }

    #[test]
    fn layout_right_aligns_showcmd() {
        let layout = layout_message_row(None, None, Some("saved"), None, Some("d"), 20);
        assert_eq!(layout.line, "saved");
        assert_eq!(layout.showcmd, "d");
        assert_eq!(layout.showcmd_col, Some(19));
    }

    #[test]
    fn layout_truncates_left_for_showcmd() {
        let layout = layout_message_row(
            None,
            None,
            Some("12345678901234567890"),
            None,
            Some("dd"),
            20,
        );
        assert_eq!(layout.line.chars().count(), 18);
        assert_eq!(layout.showcmd_col, Some(18));
    }

    #[test]
    fn layout_empty_row_has_no_column() {
        let layout = layout_message_row(None, None, None, None, None, 20);
        assert_eq!(layout.line, "");
        assert_eq!(layout.showcmd_col, None);
    }

    #[test]
    fn layout_prefers_search_over_message() {
        let layout = layout_message_row(None, Some("ar"), Some("saved"), None, None, 20);
        assert_eq!(layout.line, "/ar");
    }

    #[test]
    fn layout_shows_search_status_when_idle() {
        let layout = layout_message_row(None, None, None, Some("/ar [1/3]"), None, 20);
        assert_eq!(layout.line, "/ar [1/3]");
    }

    #[test]
    fn layout_prefers_message_over_search_status() {
        let layout = layout_message_row(None, None, Some("saved"), Some("/ar [1/3]"), None, 20);
        assert_eq!(layout.line, "saved");
    }

    #[test]
    fn search_spans_tint_current() {
        let lines = lines_of("ar ar");
        let spans = vec![
            SearchSpan {
                row: 0,
                start: 0,
                end: 2,
                current: true,
            },
            SearchSpan {
                row: 0,
                start: 3,
                end: 5,
                current: false,
            },
        ];
        let painted = search_highlight_spans(&spans, &lines, 0, &viewport());
        assert_eq!(painted.len(), 2);
        assert_eq!(painted[0].tint, Tint::SearchCurrent);
        assert_eq!(painted[0].text, "ar");
        assert_eq!(painted[1].tint, Tint::SearchMatch);
        assert_eq!(painted[1].from, 3);
    }

    #[test]
    fn search_spans_skip_outside_window() {
        let lines = lines_of("ar\nar");
        let spans = vec![
            SearchSpan {
                row: 0,
                start: 0,
                end: 2,
                current: false,
            },
            SearchSpan {
                row: 5,
                start: 0,
                end: 2,
                current: false,
            },
            SearchSpan {
                row: 1,
                start: 2,
                end: 2,
                current: false,
            },
        ];
        let painted = search_highlight_spans(&spans, &lines, 0, &viewport());
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].screen_row, 0);
    }

    #[test]
    fn prompt_cursor_absent_without_modes() {
        assert_eq!(prompt_cursor_col(None, None, 20, None), None);
    }

    #[test]
    fn prompt_cursor_sits_after_prefix_when_empty() {
        assert_eq!(prompt_cursor_col(Some(""), None, 20, None), Some(1));
        assert_eq!(prompt_cursor_col(None, Some(""), 20, None), Some(1));
    }

    #[test]
    fn prompt_cursor_follows_typed_text() {
        assert_eq!(prompt_cursor_col(None, Some("ar"), 20, None), Some(3));
        assert_eq!(prompt_cursor_col(Some("wq"), None, 20, None), Some(3));
    }

    #[test]
    fn prompt_cursor_counts_chars_not_bytes() {
        assert_eq!(prompt_cursor_col(None, Some("éx"), 20, None), Some(3));
    }

    #[test]
    fn prompt_cursor_prefers_command() {
        assert_eq!(prompt_cursor_col(Some("w"), Some("ar"), 20, None), Some(2));
    }

    #[test]
    fn prompt_cursor_clamps_to_visible_row() {
        assert_eq!(prompt_cursor_col(None, Some("abcdef"), 5, None), Some(4));
        assert_eq!(prompt_cursor_col(Some(""), None, 5, Some("dd")), Some(1));
    }
}
