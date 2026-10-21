use anyhow::{Context, Result};
use crossterm::{
    cursor::{MoveTo, SetCursorStyle},
    execute,
    style::SetForegroundColor,
    terminal::{BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate},
};
use std::io::{Stdout, Write};

use crate::frame::{CursorStyle, Frame, PickerMenu, SearchSpan, Selection};
use crate::paint;
use crate::statusbar::{draw_status, StatusInfo};
use crate::theme::{MatchColors, Theme};
use kawi_editor::Viewport;
use kawi_syntax::{SyntaxGroup, SyntaxSpan};

use crate::display;

pub fn draw(out: &mut Stdout, frame: &Frame) -> Result<()> {
    execute!(out, BeginSynchronizedUpdate).context("begin synchronized update")?;
    let contents = match frame.picker.as_ref() {
        Some(picker) => draw_picker(out, frame, picker),
        None => draw_contents(out, frame),
    };
    let finished = execute!(out, EndSynchronizedUpdate).context("end synchronized update");
    contents.and(finished)
}

fn draw_contents(out: &mut Stdout, frame: &Frame) -> Result<()> {
    paint::apply_base(out, frame.theme)?;
    draw_visible_lines(out, frame)?;
    draw_filler_rows(out, frame)?;
    draw_highlight(out, frame, frame.theme)?;
    match frame.pager {
        Some(lines) => draw_pager(out, frame, lines)?,
        None => {
            draw_status_row(out, frame.status, frame.viewport, frame.theme)?;
            draw_message_row(out, frame)?;
        }
    }
    draw_completion(out, frame)?;
    apply_cursor_style(out, frame.cursor_style)?;
    place_cursor(out, frame)?;
    flush(out)?;
    Ok(())
}

fn draw_picker(out: &mut Stdout, frame: &Frame, picker: &PickerMenu<'_>) -> Result<()> {
    paint::apply_base(out, frame.theme)?;
    execute!(out, Clear(ClearType::All)).context("clear picker")?;
    draw_picker_prompt(out, frame, picker)?;
    draw_picker_rows(out, frame, picker)?;
    draw_picker_footer(out, frame, picker)?;
    apply_cursor_style(out, CursorStyle::Bar)?;
    place_picker_cursor(out, frame, picker)?;
    flush(out)?;
    Ok(())
}

fn draw_picker_prompt(out: &mut Stdout, frame: &Frame, picker: &PickerMenu<'_>) -> Result<()> {
    execute!(
        out,
        MoveTo(0, 0),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(frame.theme.line_number)
    )
    .context("prepare picker prompt")?;
    let prompt = format!("{}  {}", picker.title, picker.query);
    write!(
        out,
        "{}",
        truncate_to_width(&prompt, frame.viewport.width())
    )
    .context("write picker prompt")?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn draw_picker_rows(out: &mut Stdout, frame: &Frame, picker: &PickerMenu<'_>) -> Result<()> {
    let available = frame.viewport.height().saturating_sub(2);
    if available == 0 {
        return Ok(());
    }
    let start = picker_start(available, picker.selected);
    let width = frame.viewport.width();
    for offset in 0..available {
        let index = start + offset;
        let row = offset.saturating_add(1);
        execute!(
            out,
            MoveTo(0, saturate_u16(row)),
            Clear(ClearType::CurrentLine)
        )
        .context("clear picker row")?;
        let Some(item) = picker.items.get(index) else {
            continue;
        };
        if picker.selected == Some(index) {
            paint::apply_match(out, &frame.theme.selection)?;
        }
        write!(out, "{}", truncate_to_width(item, width)).context("write picker row")?;
        paint::restore_base(out, frame.theme)?;
    }
    Ok(())
}

fn draw_picker_footer(out: &mut Stdout, frame: &Frame, picker: &PickerMenu<'_>) -> Result<()> {
    let height = frame.viewport.height();
    if height == 0 {
        return Ok(());
    }
    let row = height.saturating_sub(1);
    execute!(
        out,
        MoveTo(0, saturate_u16(row)),
        Clear(ClearType::CurrentLine)
    )
    .context("clear picker footer")?;
    let footer = picker_footer(picker);
    paint::apply_match(out, &frame.theme.status)?;
    write!(
        out,
        "{}",
        truncate_to_width(&footer, frame.viewport.width())
    )
    .context("write picker footer")?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn picker_footer(picker: &PickerMenu<'_>) -> String {
    if let Some(status) = picker.status {
        return status.to_string();
    }
    if picker.loading {
        return "Searching...".to_string();
    }
    if picker.query.is_empty() && picker.items.is_empty() {
        return "Type to search  Esc cancel".to_string();
    }
    format!(
        "{} results  Enter open  Tab next  Esc cancel",
        picker.items.len()
    )
}

fn picker_start(available: usize, selected: Option<usize>) -> usize {
    match selected {
        Some(index) if index >= available => index.saturating_add(1).saturating_sub(available),
        _ => 0,
    }
}

fn place_picker_cursor(out: &mut Stdout, frame: &Frame, picker: &PickerMenu<'_>) -> Result<()> {
    let width = frame.viewport.width().max(1);
    let prefix = picker.title.chars().count().saturating_add(2);
    let column = prefix.saturating_add(picker.cursor).min(width - 1);
    execute!(out, MoveTo(saturate_u16(column), 0)).context("place picker cursor")?;
    Ok(())
}

fn draw_visible_lines(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let gutter_width = gutter_width(frame);
    for (screen_row, line) in frame.lines.iter().enumerate().take(frame.viewport.height()) {
        draw_one_line(out, frame, screen_row, line, gutter_width)?;
    }
    Ok(())
}

fn draw_one_line(
    out: &mut Stdout,
    frame: &Frame,
    screen_row: usize,
    text: &str,
    gutter_width: usize,
) -> Result<()> {
    execute!(out, MoveTo(0, saturate_u16(screen_row))).context("move to line start")?;
    if let Some(gutter) = frame.gutter.get(screen_row) {
        execute!(out, SetForegroundColor(frame.theme.line_number))
            .context("apply line number color")?;
        write!(out, "{gutter}").context("write gutter")?;
        paint::restore_base(out, frame.theme)?;
    }
    let width = frame.viewport.width().saturating_sub(gutter_width);
    let end = frame.leftcol.saturating_add(width);
    let text = display::slice_screen(text, frame.leftcol, end, frame.tabstop);
    write!(out, "{}", text).context("write line")?;
    erase_row_tail(out)?;
    Ok(())
}

fn erase_row_tail(out: &mut Stdout) -> Result<()> {
    execute!(out, Clear(ClearType::UntilNewLine)).context("erase row tail")?;
    Ok(())
}

fn draw_filler_rows(out: &mut Stdout, frame: &Frame) -> Result<()> {
    for screen_row in frame.lines.len()..frame.viewport.height() {
        execute!(
            out,
            MoveTo(0, saturate_u16(screen_row)),
            SetForegroundColor(frame.theme.non_text)
        )
        .context("move to filler row")?;
        write!(out, "~").context("write filler")?;
        paint::restore_base(out, frame.theme)?;
        erase_row_tail(out)?;
    }
    Ok(())
}

fn gutter_width(frame: &Frame) -> usize {
    frame.gutter.first().map_or(0, |line| line.chars().count())
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

pub struct SpanView<'a> {
    pub window: &'a [String],
    pub top: usize,
    pub viewport: &'a Viewport,
    pub gutter: usize,
    pub leftcol: usize,
    pub tabstop: usize,
}

impl SpanView<'_> {
    fn text_width(&self) -> usize {
        self.viewport.width().saturating_sub(self.gutter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Selection,
    SearchMatch,
    SearchCurrent,
    Substitute,
    Syntax(SyntaxGroup),
}

pub fn highlight_spans(
    view: &SpanView,
    selection: Option<Selection>,
    cursor: Option<(usize, usize)>,
) -> Vec<HighlightSpan> {
    let range = match selection {
        Some(Selection::Range { start, end }) => Some((start, end)),
        Some(selection @ Selection::Block { .. }) => return block_spans(view, selection, cursor),
        None => None,
    };
    let Some(range) = range else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    for screen_row in 0..view.window.len().min(view.viewport.height()) {
        collect_row_spans(&mut spans, view, screen_row, range, cursor);
    }
    spans
}

fn block_spans(
    view: &SpanView,
    selection: Selection,
    cursor: Option<(usize, usize)>,
) -> Vec<HighlightSpan> {
    let Selection::Block {
        top: block_top,
        left,
        bottom: block_bottom,
        right,
        ragged,
    } = selection
    else {
        return Vec::new();
    };
    let right_edge = view.leftcol.saturating_add(view.text_width());
    let mut spans = Vec::new();
    for screen_row in 0..view.window.len().min(view.viewport.height()) {
        let buffer_row = view.top.saturating_add(screen_row);
        if buffer_row < block_top || buffer_row > block_bottom {
            continue;
        }
        let Some(text) = view.window.get(screen_row) else {
            continue;
        };
        let to = if ragged {
            char_len(text)
        } else {
            right.saturating_add(1)
        };
        let cell = cursor
            .and_then(|(row, col)| (row == buffer_row).then_some(col.saturating_add(view.leftcol)));
        for (from, to) in cut_cell(left, to, cell) {
            let from = from.max(view.leftcol);
            let to = to.min(right_edge);
            if from >= to {
                continue;
            }
            let mut text = display::slice_screen(text, from, to, view.tabstop);
            if !ragged {
                let missing = to.saturating_sub(from).saturating_sub(text.chars().count());
                text.push_str(&" ".repeat(missing));
            }
            spans.push(HighlightSpan {
                screen_row,
                from: from.saturating_sub(view.leftcol),
                text,
                tint: Tint::Selection,
            });
        }
    }
    spans
}

fn collect_row_spans(
    spans: &mut Vec<HighlightSpan>,
    view: &SpanView,
    screen_row: usize,
    range: ((usize, usize), (usize, usize)),
    cursor: Option<(usize, usize)>,
) {
    let Some(text) = view.window.get(screen_row) else {
        return;
    };
    let buffer_row = view.top.saturating_add(screen_row);
    let cell = cursor
        .and_then(|(row, col)| (row == buffer_row).then_some(col.saturating_add(view.leftcol)));
    for (from, to) in selected_cells(buffer_row, text, range, cell, view.tabstop) {
        if let Some(span) = clipped_span(view, screen_row, text, from, to, Tint::Selection) {
            spans.push(span);
        }
    }
}

fn selected_cells(
    buffer_row: usize,
    text: &str,
    range: ((usize, usize), (usize, usize)),
    cursor: Option<usize>,
    tabstop: usize,
) -> Vec<(usize, usize)> {
    let ((start_row, start_col), (end_row, end_col)) = range;
    if buffer_row < start_row || buffer_row > end_row {
        return Vec::new();
    }
    let line_len = char_len(text);
    let (from, to) = row_selection(buffer_row, line_len, start_row, start_col, end_row, end_col);
    let (from, to) = if line_len == 0 {
        (from, to)
    } else {
        (
            display::screen_col(text, from, tabstop),
            display::screen_col(text, to, tabstop),
        )
    };
    cut_cell(from, to, cursor)
}

fn row_selection(
    buffer_row: usize,
    line_len: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> (usize, usize) {
    if line_len == 0 {
        return (0, 1);
    }
    let from = if buffer_row == start_row {
        start_col.min(line_len)
    } else {
        0
    };
    let to = if buffer_row == end_row {
        end_col.saturating_add(1).min(line_len)
    } else {
        line_len
    };
    (from, to)
}

fn cut_cell(from: usize, to: usize, cell: Option<usize>) -> Vec<(usize, usize)> {
    match cell {
        Some(cell) if from <= cell && cell < to => {
            let mut parts = Vec::new();
            if from < cell {
                parts.push((from, cell));
            }
            if cell + 1 < to {
                parts.push((cell + 1, to));
            }
            parts
        }
        _ => vec![(from, to)],
    }
}

fn clipped_span(
    view: &SpanView,
    screen_row: usize,
    text: &str,
    from: usize,
    to: usize,
    tint: Tint,
) -> Option<HighlightSpan> {
    let left = view.leftcol;
    let right = left.saturating_add(view.text_width());
    let from = from.max(left);
    let to = to.min(right);
    if from >= to {
        return None;
    }
    let mut text = display::slice_screen(text, from, to, view.tabstop);
    if text.is_empty() {
        text.push(' ');
    }
    Some(HighlightSpan {
        screen_row,
        from: from.saturating_sub(left),
        text,
        tint,
    })
}

pub fn search_highlight_spans(spans: &[SearchSpan], view: &SpanView) -> Vec<HighlightSpan> {
    span_highlights(spans, view, |span| search_tint(span.current))
}

fn preview_highlight_spans(spans: &[SearchSpan], view: &SpanView) -> Vec<HighlightSpan> {
    span_highlights(spans, view, |_| Tint::Substitute)
}

pub fn syntax_highlight_spans(spans: &[SyntaxSpan], view: &SpanView) -> Vec<HighlightSpan> {
    spans
        .iter()
        .filter_map(|span| {
            span_segment_at(
                span.row,
                span.start,
                span.end,
                view,
                Tint::Syntax(span.group),
            )
        })
        .collect()
}

fn span_highlights(
    spans: &[SearchSpan],
    view: &SpanView,
    tint: impl Fn(&SearchSpan) -> Tint,
) -> Vec<HighlightSpan> {
    spans
        .iter()
        .filter_map(|span| span_segment(span, view, tint(span)))
        .collect()
}

fn span_segment(span: &SearchSpan, view: &SpanView, tint: Tint) -> Option<HighlightSpan> {
    span_segment_at(span.row, span.start, span.end, view, tint)
}

fn span_segment_at(
    row: usize,
    start: usize,
    end: usize,
    view: &SpanView,
    tint: Tint,
) -> Option<HighlightSpan> {
    if row < view.top {
        return None;
    }
    let screen_row = row.saturating_sub(view.top);
    let text = view.window.get(screen_row)?;
    if screen_row >= view.viewport.height() {
        return None;
    }
    let from = display::screen_col(text, start, view.tabstop);
    let to = display::screen_col(text, end, view.tabstop);
    clipped_span(view, screen_row, text, from, to, tint)
}

fn search_tint(current: bool) -> Tint {
    if current {
        return Tint::SearchCurrent;
    }
    Tint::SearchMatch
}

fn draw_highlight(out: &mut Stdout, frame: &Frame, theme: &Theme) -> Result<()> {
    let gutter_width = gutter_width(frame);
    let view = SpanView {
        window: frame.lines,
        top: frame.viewport.top(),
        viewport: frame.viewport,
        gutter: gutter_width,
        leftcol: frame.leftcol,
        tabstop: frame.tabstop,
    };
    for span in syntax_highlight_spans(frame.syntax_spans, &view) {
        paint_span(out, &span, gutter_width, theme)?;
    }
    for span in search_highlight_spans(frame.search_spans, &view) {
        paint_span(out, &span, gutter_width, theme)?;
    }
    for span in preview_highlight_spans(frame.preview_spans, &view) {
        paint_span(out, &span, gutter_width, theme)?;
    }
    for span in highlight_spans(
        &view,
        frame.selection,
        visual_cursor_cell(frame, gutter_width),
    ) {
        paint_span(out, &span, gutter_width, theme)?;
    }
    Ok(())
}

fn visual_cursor_cell(frame: &Frame, gutter_width: usize) -> Option<(usize, usize)> {
    frame.selection.as_ref()?;
    Some((frame.row, frame.cursor_col.saturating_sub(gutter_width)))
}

fn paint_span(
    out: &mut Stdout,
    span: &HighlightSpan,
    gutter_width: usize,
    theme: &Theme,
) -> Result<()> {
    move_to_segment(out, span.from.saturating_add(gutter_width), span.screen_row)?;
    match span.tint {
        Tint::Selection => paint_tinted(out, &span.text, &theme.selection, theme),
        Tint::SearchMatch => paint_tinted(out, &span.text, &theme.search_match, theme),
        Tint::SearchCurrent => paint_tinted(out, &span.text, &theme.search_current, theme),
        Tint::Substitute => paint_tinted(out, &span.text, &theme.substitute, theme),
        Tint::Syntax(group) => {
            let colors = MatchColors {
                foreground: theme.syntax.color(group),
                background: theme.base.background,
            };
            paint_tinted(out, &span.text, &colors, theme)
        }
    }
}

fn paint_tinted(out: &mut Stdout, text: &str, colors: &MatchColors, theme: &Theme) -> Result<()> {
    paint::apply_match(out, colors)?;
    write_segment(out, text)?;
    paint::restore_base(out, theme)?;
    Ok(())
}

fn move_to_segment(out: &mut Stdout, col: usize, screen_row: usize) -> Result<()> {
    execute!(out, MoveTo(saturate_u16(col), saturate_u16(screen_row)))
        .context("move to segment")?;
    Ok(())
}

fn write_segment(out: &mut Stdout, text: &str) -> Result<()> {
    write!(out, "{text}").context("write segment")?;
    Ok(())
}

fn draw_status_row(
    out: &mut Stdout,
    status: &StatusInfo,
    viewport: &Viewport,
    theme: &Theme,
) -> Result<()> {
    draw_status(out, status, viewport.width(), viewport.height(), theme)
}

fn draw_completion(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let Some(menu) = frame.completion.as_ref() else {
        return Ok(());
    };
    if menu.items.is_empty() {
        return Ok(());
    }
    let available = frame.viewport.height().saturating_add(1);
    let shown = menu.items.len().min(available);
    let start = menu_start(shown, menu.selected);
    let width = frame.viewport.width();
    for offset in 0..shown {
        let index = start + offset;
        let row = frame
            .viewport
            .height()
            .saturating_sub(shown.saturating_sub(1).saturating_sub(offset));
        execute!(
            out,
            MoveTo(0, saturate_u16(row)),
            Clear(ClearType::CurrentLine)
        )
        .context("clear completion row")?;
        if menu.selected == Some(index) {
            paint::apply_match(out, &frame.theme.selection)?;
        }
        write!(out, "{}", truncate_to_width(&menu.items[index], width))
            .context("write completion row")?;
        paint::restore_base(out, frame.theme)?;
    }
    Ok(())
}

fn menu_start(shown: usize, selected: Option<usize>) -> usize {
    match selected {
        Some(index) if index >= shown => index.saturating_add(1).saturating_sub(shown),
        _ => 0,
    }
}

const PAGER_PROMPT: &str = "Press ENTER or type command to continue";

fn draw_pager(out: &mut Stdout, frame: &Frame, lines: &[String]) -> Result<()> {
    let width = frame.viewport.width();
    let available = frame.viewport.height().saturating_add(1);
    let visible = &lines[lines.len().saturating_sub(available)..];
    for (index, line) in visible.iter().enumerate() {
        let row = frame
            .viewport
            .height()
            .saturating_sub(visible.len().saturating_sub(1).saturating_sub(index));
        execute!(
            out,
            MoveTo(0, saturate_u16(row)),
            Clear(ClearType::CurrentLine)
        )
        .context("clear pager row")?;
        write!(out, "{}", truncate_to_width(line, width)).context("write pager line")?;
    }
    let prompt_row = frame.viewport.height().saturating_add(1);
    execute!(
        out,
        MoveTo(0, saturate_u16(prompt_row)),
        Clear(ClearType::CurrentLine)
    )
    .context("clear pager prompt")?;
    write!(out, "{}", truncate_to_width(PAGER_PROMPT, width)).context("write pager prompt")?;
    Ok(())
}

fn draw_message_row(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let layout = layout_message_row(
        frame.command,
        frame.search,
        frame.search_delimiter,
        frame.message,
        frame.search_status,
        frame.showcmd.as_deref(),
        frame.viewport.width(),
    );
    move_to_message_row(out, frame.viewport)?;
    write!(out, "{}", layout.line).context("write message row")?;
    erase_row_tail(out)?;
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
    search_delimiter: char,
    message: Option<&str>,
    search_status: Option<&str>,
    showcmd: Option<&str>,
    width: usize,
) -> MessageLayout {
    let show = showcmd.unwrap_or("");
    let left = message_left(command, search, search_delimiter, message, search_status);
    MessageLayout {
        line: truncate_to_width(&left, left_width(width, show)),
        showcmd: truncate_to_width(show, width),
        showcmd_col: showcmd_column(show, width),
    }
}

fn message_left(
    command: Option<&str>,
    search: Option<&str>,
    search_delimiter: char,
    message: Option<&str>,
    search_status: Option<&str>,
) -> String {
    if let Some(text) = command {
        return format!(":{text}");
    }
    if let Some(text) = search {
        return format!("{search_delimiter}{text}");
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

fn apply_cursor_style(out: &mut Stdout, style: CursorStyle) -> Result<()> {
    execute!(out, cursor_style(style)).context("set cursor style")?;
    Ok(())
}

fn cursor_style(style: CursorStyle) -> SetCursorStyle {
    match style {
        CursorStyle::Block => SetCursorStyle::SteadyBlock,
        CursorStyle::Bar => SetCursorStyle::SteadyBar,
        CursorStyle::Underline => SetCursorStyle::SteadyUnderScore,
    }
}

fn place_cursor(out: &mut Stdout, frame: &Frame) -> Result<()> {
    if frame.pager.is_some() {
        return place_pager_cursor(out, frame);
    }
    if let Some(col) = prompt_cursor_col(
        frame.command,
        frame.search,
        frame.viewport.width(),
        frame.showcmd.as_deref(),
    ) {
        return place_prompt_cursor(out, col, frame.viewport);
    }
    place_cursor_relative(out, frame.cursor_col, frame.row, frame.viewport)
}

fn place_cursor_relative(
    out: &mut Stdout,
    cursor_col: usize,
    row: usize,
    viewport: &Viewport,
) -> Result<()> {
    let (x, y) = screen_coords(cursor_col, row, viewport);
    execute!(out, MoveTo(x, y)).context("place cursor")?;
    Ok(())
}

fn place_pager_cursor(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let width = frame.viewport.width().max(1);
    let x = PAGER_PROMPT.chars().count().min(width - 1);
    execute!(
        out,
        MoveTo(
            saturate_u16(x),
            saturate_u16(frame.viewport.height().saturating_add(1))
        )
    )
    .context("place pager cursor")?;
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

    fn span_view<'a>(lines: &'a [String], viewport: &'a Viewport) -> SpanView<'a> {
        SpanView {
            window: lines,
            top: 0,
            viewport,
            gutter: 0,
            leftcol: 0,
            tabstop: 8,
        }
    }

    fn spans_of(
        lines: &[String],
        top: usize,
        selection: Option<Selection>,
        viewport: &Viewport,
        cursor: Option<(usize, usize)>,
    ) -> Vec<HighlightSpan> {
        let view = SpanView {
            window: lines,
            top,
            viewport,
            gutter: 0,
            leftcol: 0,
            tabstop: 8,
        };
        highlight_spans(&view, selection, cursor)
    }

    #[test]
    fn cursor_style_follows_frame_kind() {
        assert_eq!(
            cursor_style(CursorStyle::Block),
            SetCursorStyle::SteadyBlock
        );
        assert_eq!(cursor_style(CursorStyle::Bar), SetCursorStyle::SteadyBar);
        assert_eq!(
            cursor_style(CursorStyle::Underline),
            SetCursorStyle::SteadyUnderScore
        );
    }

    #[test]
    fn no_selection_has_no_spans() {
        let lines = lines_of("abcdef");
        assert!(spans_of(&lines, 0, None, &viewport(), None).is_empty());
    }

    #[test]
    fn point_selection_paints_one_cell_without_cursor() {
        let lines = lines_of("abcdef");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 2),
                end: (0, 2),
            }),
            &viewport(),
            None,
        );
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].from, 2);
        assert_eq!(spans[0].text, "c");
    }

    #[test]
    fn point_selection_skips_cursor_cell() {
        let lines = lines_of("abcdef");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 2),
                end: (0, 2),
            }),
            &viewport(),
            Some((0, 2)),
        );
        assert!(spans.is_empty());
    }

    #[test]
    fn linewise_selection_skips_cursor_cell() {
        let lines = lines_of("ab\ncd");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 0),
                end: (1, 2),
            }),
            &viewport(),
            Some((0, 0)),
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].from, 1);
        assert_eq!(spans[0].text, "b");
        assert_eq!(spans[1].from, 0);
        assert_eq!(spans[1].text, "cd");
    }

    #[test]
    fn linewise_selection_has_no_eol_cell() {
        let lines = lines_of("ab\ncd");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 0),
                end: (1, 2),
            }),
            &viewport(),
            Some((1, 0)),
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "ab");
        assert_eq!(spans[1].text, "d");
    }

    #[test]
    fn split_selection_around_cursor_cell() {
        let lines = lines_of("abcdef");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 0),
                end: (0, 6),
            }),
            &viewport(),
            Some((0, 2)),
        );
        assert_eq!(spans.len(), 2);
        assert_eq!((spans[0].from, spans[0].text.as_str()), (0, "ab"));
        assert_eq!((spans[1].from, spans[1].text.as_str()), (3, "def"));
    }

    #[test]
    fn empty_line_inside_selection_paints_one_cell() {
        let lines = lines_of("ab\n\ncd");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 0),
                end: (2, 2),
            }),
            &viewport(),
            Some((2, 0)),
        );
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "ab");
        assert_eq!((spans[1].from, spans[1].text.as_str()), (0, " "));
        assert_eq!(spans[2].text, "d");
    }

    #[test]
    fn empty_cursor_line_paints_nothing() {
        let lines = lines_of("ab\n\ncd");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 0),
                end: (2, 0),
            }),
            &viewport(),
            Some((2, 0)),
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "ab");
        assert_eq!(spans[1].text, " ");
    }

    #[test]
    fn charwise_selection_has_no_eol_cell() {
        let lines = lines_of("abcdef\nghij");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 0),
                end: (1, 2),
            }),
            &viewport(),
            Some((1, 2)),
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "abcdef");
        assert_eq!(spans[1].text, "gh");
    }

    #[test]
    fn cursor_past_line_end_adds_no_cell() {
        let lines = lines_of("abc");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 0),
                end: (0, 3),
            }),
            &viewport(),
            Some((0, 3)),
        );
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "abc");
    }

    #[test]
    fn range_selection_spans_chars() {
        let lines = lines_of("abcdef");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 1),
                end: (0, 3),
            }),
            &viewport(),
            None,
        );
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].screen_row, 0);
        assert_eq!(spans[0].from, 1);
        assert_eq!(spans[0].text, "bcd");
    }

    #[test]
    fn multiline_selection_spans_rows() {
        let lines = lines_of("ab\ncd\nef");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Range {
                start: (0, 1),
                end: (2, 1),
            }),
            &viewport(),
            Some((2, 1)),
        );
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "b");
        assert_eq!(spans[1].text, "cd");
        assert_eq!(spans[2].text, "e");
    }

    #[test]
    fn block_selection_skips_cursor_cell() {
        let lines = lines_of("abcd\nefgh");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Block {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
                ragged: false,
            }),
            &viewport(),
            Some((1, 1)),
        );
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "abc");
        assert_eq!((spans[1].from, spans[1].text.as_str()), (0, "e"));
        assert_eq!((spans[2].from, spans[2].text.as_str()), (2, "g"));
    }

    #[test]
    fn block_selection_pads_short_rows() {
        let lines = lines_of("ab\nabcd");
        let spans = spans_of(
            &lines,
            0,
            Some(Selection::Block {
                top: 0,
                left: 0,
                bottom: 1,
                right: 3,
                ragged: false,
            }),
            &viewport(),
            None,
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "ab  ");
        assert_eq!(spans[1].text, "abcd");
    }

    #[test]
    fn layout_prefers_command_over_message() {
        let layout = layout_message_row(Some("w"), None, '/', Some("saved"), None, None, 20);
        assert_eq!(layout.line, ":w");
        assert_eq!(layout.showcmd_col, None);
    }

    #[test]
    fn layout_right_aligns_showcmd() {
        let layout = layout_message_row(None, None, '/', Some("saved"), None, Some("d"), 20);
        assert_eq!(layout.line, "saved");
        assert_eq!(layout.showcmd, "d");
        assert_eq!(layout.showcmd_col, Some(19));
    }

    #[test]
    fn layout_prefixes_backward_search_prompt() {
        let layout = layout_message_row(None, Some("foo"), '?', None, None, None, 20);
        assert_eq!(layout.line, "?foo");
    }

    #[test]
    fn layout_truncates_left_for_showcmd() {
        let layout = layout_message_row(
            None,
            None,
            '/',
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
        let layout = layout_message_row(None, None, '/', None, None, None, 20);
        assert_eq!(layout.line, "");
        assert_eq!(layout.showcmd_col, None);
    }

    #[test]
    fn layout_prefers_search_over_message() {
        let layout = layout_message_row(None, Some("ar"), '/', Some("saved"), None, None, 20);
        assert_eq!(layout.line, "/ar");
    }

    #[test]
    fn layout_shows_search_status_when_idle() {
        let layout = layout_message_row(None, None, '/', None, Some("/ar [1/3]"), None, 20);
        assert_eq!(layout.line, "/ar [1/3]");
    }

    #[test]
    fn layout_prefers_message_over_search_status() {
        let layout =
            layout_message_row(None, None, '/', Some("saved"), Some("/ar [1/3]"), None, 20);
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
        let viewport = viewport();
        let view = span_view(&lines, &viewport);
        let painted = search_highlight_spans(&spans, &view);
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
        let viewport = viewport();
        let view = span_view(&lines, &viewport);
        let painted = search_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].screen_row, 0);
    }

    #[test]
    fn preview_spans_use_substitute_tint() {
        let lines = lines_of("orange");
        let spans = vec![SearchSpan {
            row: 0,
            start: 0,
            end: 6,
            current: true,
        }];
        let viewport = viewport();
        let view = span_view(&lines, &viewport);
        let painted = preview_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].tint, Tint::Substitute);
        assert_eq!(painted[0].text, "orange");
    }

    #[test]
    fn spans_shift_by_leftcol() {
        let lines = lines_of("0123456789");
        let spans = vec![SearchSpan {
            row: 0,
            start: 0,
            end: 10,
            current: false,
        }];
        let viewport = viewport();
        let mut view = span_view(&lines, &viewport);
        view.leftcol = 3;
        let painted = search_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].from, 0);
        assert_eq!(painted[0].text, "3456789");
    }

    #[test]
    fn spans_clip_to_text_width_with_gutter() {
        let lines = lines_of("012345678901234567890123");
        let spans = vec![SearchSpan {
            row: 0,
            start: 0,
            end: 24,
            current: false,
        }];
        let viewport = viewport();
        let mut view = span_view(&lines, &viewport);
        view.gutter = 4;
        view.leftcol = 5;
        let painted = search_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].from, 0);
        assert_eq!(painted[0].text, "5678901234567890");
    }

    #[test]
    fn spans_expand_tabs() {
        let lines = lines_of("\tab");
        let spans = vec![SearchSpan {
            row: 0,
            start: 0,
            end: 2,
            current: false,
        }];
        let viewport = viewport();
        let view = span_view(&lines, &viewport);
        let painted = search_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].text, "        a");
    }

    #[test]
    fn selection_spans_expand_tabs() {
        let lines = lines_of("a\tb");
        let viewport = viewport();
        let view = span_view(&lines, &viewport);
        let painted = highlight_spans(
            &view,
            Some(Selection::Range {
                start: (0, 0),
                end: (0, 2),
            }),
            None,
        );
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].text, "a       b");
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

    #[test]
    fn syntax_spans_use_group_tints() {
        let lines = lines_of("fn main");
        let viewport = Viewport::new(10, 20);
        let view = span_view(&lines, &viewport);
        let spans = vec![
            SyntaxSpan {
                row: 0,
                start: 0,
                end: 2,
                group: SyntaxGroup::Keyword,
            },
            SyntaxSpan {
                row: 0,
                start: 3,
                end: 7,
                group: SyntaxGroup::Function,
            },
        ];
        let painted = syntax_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 2);
        assert_eq!(painted[0].tint, Tint::Syntax(SyntaxGroup::Keyword));
        assert_eq!(painted[0].text, "fn");
        assert_eq!(painted[1].tint, Tint::Syntax(SyntaxGroup::Function));
        assert_eq!(painted[1].from, 3);
    }

    #[test]
    fn syntax_spans_shift_by_leftcol() {
        let lines = lines_of("0123456789");
        let viewport = Viewport::new(10, 20);
        let mut view = span_view(&lines, &viewport);
        view.leftcol = 4;
        let spans = vec![SyntaxSpan {
            row: 0,
            start: 0,
            end: 10,
            group: SyntaxGroup::String,
        }];
        let painted = syntax_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].from, 0);
        assert_eq!(painted[0].text, "456789");
    }

    #[test]
    fn syntax_spans_expand_tabs() {
        let lines = lines_of("\tab");
        let viewport = Viewport::new(10, 20);
        let view = span_view(&lines, &viewport);
        let spans = vec![SyntaxSpan {
            row: 0,
            start: 0,
            end: 2,
            group: SyntaxGroup::Variable,
        }];
        let painted = syntax_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].text, "        a");
    }
}
