use anyhow::{Context, Result};
use crossterm::{
    cursor::{MoveTo, SetCursorStyle, Show},
    execute,
    style::{Attribute, Color, SetAttribute, SetForegroundColor},
    terminal::{BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate},
};
use std::io::{Stdout, Write};

use crate::frame::{
    CursorStyle, DiagnosticSpan, Frame, PaneFrame, PickerMenu, PromptMenu, ReplaceMenu,
    ReplaceStage, SearchSpan, Selection, TaskPanel,
};
use crate::overlay::{PANEL_BORDER, PANEL_PADDING};
use crate::paint;
use crate::statusbar::draw_status;
use crate::text::{count_label, display_width, truncate_to_display_width};
use crate::theme::{MatchColors, Theme};
use kawi_editor::Viewport;
use kawi_syntax::{SyntaxHighlight, SyntaxSpan};

use crate::display;

mod leader;
mod terminal;

use terminal::{draw_terminal_panel, place_terminal_cursor};

pub fn draw(out: &mut Stdout, frame: &Frame) -> Result<()> {
    execute!(out, BeginSynchronizedUpdate).context("begin synchronized update")?;
    let contents = match (frame.replace.as_ref(), frame.picker.as_ref()) {
        (Some(replace), _) => draw_replace(out, frame, replace),
        (None, Some(picker)) => draw_picker(out, frame, picker),
        (None, None) => draw_contents(out, frame),
    };
    let finished = execute!(out, EndSynchronizedUpdate).context("end synchronized update");
    contents.and(finished)
}

fn draw_contents(out: &mut Stdout, frame: &Frame) -> Result<()> {
    paint::apply_base(out, frame.theme)?;
    for pane in frame.panes {
        draw_pane(out, frame, pane)?;
    }
    draw_separators(out, frame)?;
    draw_task_panel(out, frame)?;
    draw_terminal_panel(out, frame)?;
    match frame.pager {
        Some(lines) => draw_pager(out, frame, lines)?,
        None => draw_message_row(out, frame)?,
    }
    draw_completion(out, frame)?;
    draw_overlay(out, frame)?;
    if let Some(terminal) = frame.terminal.as_ref().filter(|terminal| terminal.focused) {
        place_terminal_cursor(out, terminal)?;
    } else {
        execute!(out, Show).context("show editor cursor")?;
        apply_cursor_style(out, frame.cursor_style)?;
        place_cursor(out, frame)?;
    }
    flush(out)?;
    Ok(())
}

fn draw_pane(out: &mut Stdout, frame: &Frame, pane: &PaneFrame<'_>) -> Result<()> {
    draw_visible_lines(out, pane, frame.theme)?;
    draw_filler_rows(out, pane, frame.theme)?;
    draw_highlight(out, pane, frame.theme)?;
    if pane.rect.height == 0 {
        return Ok(());
    }
    let row = pane
        .rect
        .y
        .saturating_add(pane.rect.height.saturating_sub(1));
    draw_status(
        out,
        pane.status,
        pane.rect.x,
        row,
        pane.rect.width,
        pane.active,
        frame.theme,
    )
}

fn draw_task_panel(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let Some(panel) = frame.task.as_ref() else {
        return Ok(());
    };
    draw_task_header(out, frame, panel)?;
    draw_task_lines(out, frame, panel)?;
    Ok(())
}

fn draw_task_header(out: &mut Stdout, frame: &Frame, panel: &TaskPanel<'_>) -> Result<()> {
    execute!(
        out,
        MoveTo(saturate_u16(panel.rect.x), saturate_u16(panel.rect.y)),
        Clear(ClearType::CurrentLine)
    )
    .context("prepare task header")?;
    paint::apply_match(out, &frame.theme.status)?;
    let header = format!("{}  {}", panel.title, panel.status);
    write_padded(out, &header, panel.rect.width)?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn draw_task_lines(out: &mut Stdout, frame: &Frame, panel: &TaskPanel<'_>) -> Result<()> {
    for offset in 0..panel.rect.height.saturating_sub(1) {
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        execute!(
            out,
            MoveTo(saturate_u16(panel.rect.x), saturate_u16(row)),
            Clear(ClearType::CurrentLine)
        )
        .context("clear task line")?;
        let Some(line) = panel.lines.get(offset) else {
            continue;
        };
        if line.stderr {
            execute!(out, SetForegroundColor(frame.theme.diagnostic_warning))
                .context("apply task stderr color")?;
        }
        write!(out, "{}", truncate_to_width(line.text, panel.rect.width))
            .context("write task line")?;
        paint::restore_base(out, frame.theme)?;
    }
    Ok(())
}

fn draw_separators(out: &mut Stdout, frame: &Frame) -> Result<()> {
    if frame.separators.is_empty() {
        return Ok(());
    }
    execute!(out, SetForegroundColor(frame.theme.non_text)).context("apply separator color")?;
    for separator in frame.separators {
        for offset in 0..separator.height {
            execute!(
                out,
                MoveTo(
                    saturate_u16(separator.x),
                    saturate_u16(separator.y.saturating_add(offset))
                )
            )
            .context("move to separator")?;
            write!(out, "│").context("write separator")?;
        }
    }
    paint::restore_base(out, frame.theme)
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
    if picker.replace_hint {
        return format!(
            "{} results  Enter open  Ctrl-R replace  Tab next  Esc cancel",
            picker.items.len()
        );
    }
    if picker.file_hint {
        return format!(
            "{} results  Enter open  Ctrl-A new  Ctrl-R rename  Ctrl-D delete  Tab next  Esc cancel",
            picker.items.len()
        );
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

fn draw_replace(out: &mut Stdout, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
    paint::apply_base(out, frame.theme)?;
    execute!(out, Clear(ClearType::All)).context("clear replace")?;
    draw_replace_prompt(out, frame, replace)?;
    draw_replace_rows(out, frame, replace)?;
    draw_replace_footer(out, frame, replace)?;
    apply_cursor_style(out, CursorStyle::Bar)?;
    place_replace_cursor(out, frame, replace)?;
    flush(out)?;
    Ok(())
}

fn draw_replace_prompt(out: &mut Stdout, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
    execute!(
        out,
        MoveTo(0, 0),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(frame.theme.line_number)
    )
    .context("prepare replace prompt")?;
    let prompt = format!("Replace  {}", replace.query);
    write!(
        out,
        "{}",
        truncate_to_width(&prompt, frame.viewport.width())
    )
    .context("write replace prompt")?;
    if replace.stage == ReplaceStage::Search {
        paint::restore_base(out, frame.theme)?;
        return Ok(());
    }
    execute!(
        out,
        MoveTo(0, 1),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(frame.theme.line_number)
    )
    .context("prepare replacement prompt")?;
    let prompt = format!("with: {}", replace.replacement);
    write!(
        out,
        "{}",
        truncate_to_width(&prompt, frame.viewport.width())
    )
    .context("write replacement prompt")?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn draw_replace_rows(out: &mut Stdout, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
    let first_row = replace_rows_start(replace.stage);
    let available = frame.viewport.height().saturating_sub(first_row + 1);
    if available == 0 {
        return Ok(());
    }
    let start = picker_start(available, replace.selected);
    let width = frame.viewport.width();
    for offset in 0..available {
        let index = start + offset;
        let row = first_row + offset;
        execute!(
            out,
            MoveTo(0, saturate_u16(row)),
            Clear(ClearType::CurrentLine)
        )
        .context("clear replace row")?;
        let Some(item) = replace.items.get(index) else {
            continue;
        };
        if replace.selected == Some(index) {
            paint::apply_match(out, &frame.theme.selection)?;
        }
        write!(out, "{}", truncate_to_width(item, width)).context("write replace row")?;
        paint::restore_base(out, frame.theme)?;
    }
    Ok(())
}

fn draw_replace_footer(out: &mut Stdout, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
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
    .context("clear replace footer")?;
    let footer = replace_footer(replace);
    paint::apply_match(out, &frame.theme.status)?;
    write!(
        out,
        "{}",
        truncate_to_width(&footer, frame.viewport.width())
    )
    .context("write replace footer")?;
    paint::restore_base(out, frame.theme)?;
    Ok(())
}

fn replace_footer(replace: &ReplaceMenu<'_>) -> String {
    if let Some(status) = replace.status {
        return status.to_string();
    }
    if replace.loading {
        return match replace.stage {
            ReplaceStage::Search => "Searching...".to_string(),
            ReplaceStage::Replacement | ReplaceStage::Preview => "Building preview...".to_string(),
        };
    }
    match replace.stage {
        ReplaceStage::Search => {
            if replace.items.is_empty() {
                return "Type to search  Enter replace  Esc cancel".to_string();
            }
            format!(
                "{} results  Enter replace  Tab next  Esc cancel",
                replace.items.len()
            )
        }
        ReplaceStage::Replacement => "Enter preview  Esc search  groups: & \\1 $1".to_string(),
        ReplaceStage::Preview => {
            let matches = count_label(replace.matches, "match", "matches");
            let files = count_label(replace.files, "file", "files");
            format!("{matches} in {files}  y apply  n/Esc back")
        }
    }
}

fn replace_rows_start(stage: ReplaceStage) -> usize {
    match stage {
        ReplaceStage::Search => 1,
        ReplaceStage::Replacement | ReplaceStage::Preview => 2,
    }
}

fn place_replace_cursor(out: &mut Stdout, frame: &Frame, replace: &ReplaceMenu<'_>) -> Result<()> {
    let width = frame.viewport.width().max(1);
    let (row, prefix) = match replace.stage {
        ReplaceStage::Search => (0, "Replace  ".chars().count()),
        ReplaceStage::Replacement | ReplaceStage::Preview => (1, "with: ".chars().count()),
    };
    let column = prefix.saturating_add(replace.cursor).min(width - 1);
    execute!(out, MoveTo(saturate_u16(column), saturate_u16(row)))
        .context("place replace cursor")?;
    Ok(())
}

fn draw_visible_lines(out: &mut Stdout, pane: &PaneFrame<'_>, theme: &Theme) -> Result<()> {
    let gutter_width = gutter_width(pane);
    let height = pane.rect.height.saturating_sub(1);
    for (screen_row, line) in pane.lines.iter().enumerate().take(height) {
        draw_one_line(out, pane, theme, screen_row, line, gutter_width)?;
    }
    Ok(())
}

fn draw_one_line(
    out: &mut Stdout,
    pane: &PaneFrame<'_>,
    theme: &Theme,
    screen_row: usize,
    text: &str,
    gutter_width: usize,
) -> Result<()> {
    execute!(
        out,
        MoveTo(
            saturate_u16(pane.rect.x),
            saturate_u16(pane.rect.y.saturating_add(screen_row))
        )
    )
    .context("move to line start")?;
    if let Some(signs) = pane.signs {
        match signs.get(screen_row).and_then(|sign| *sign) {
            Some(sign) => {
                execute!(out, SetForegroundColor(theme.sign_color(sign)))
                    .context("apply sign color")?;
                write!(out, "{}", sign.text()).context("write sign")?;
            }
            None => write!(out, " ").context("write empty sign")?,
        }
        execute!(out, SetForegroundColor(theme.line_number)).context("apply line number color")?;
    }
    if let Some(gutter) = pane.gutter.get(screen_row) {
        execute!(out, SetForegroundColor(theme.line_number)).context("apply line number color")?;
        write!(out, "{gutter}").context("write gutter")?;
    }
    if pane.signs.is_some() || pane.gutter.get(screen_row).is_some() {
        paint::restore_base(out, theme)?;
    }
    let width = pane.rect.width.saturating_sub(gutter_width);
    let end = pane.viewport.leftcol().saturating_add(width);
    let text = display::slice_screen(text, pane.viewport.leftcol(), end, pane.tabstop);
    let visible = display::screen_len(&text, pane.tabstop);
    write!(out, "{}", text).context("write line")?;
    erase_row_tail(out)?;
    draw_rulers(out, pane, theme, screen_row, gutter_width, visible)?;
    Ok(())
}

fn draw_rulers(
    out: &mut Stdout,
    pane: &PaneFrame<'_>,
    theme: &Theme,
    screen_row: usize,
    gutter_width: usize,
    visible: usize,
) -> Result<()> {
    let width = pane.rect.width.saturating_sub(gutter_width);
    let leftcol = pane.viewport.leftcol();
    for ruler in pane.rulers {
        if *ruler < leftcol || *ruler >= leftcol.saturating_add(width) || visible > *ruler {
            continue;
        }
        let column = pane
            .rect
            .x
            .saturating_add(gutter_width)
            .saturating_add(ruler.saturating_sub(leftcol));
        execute!(
            out,
            MoveTo(
                saturate_u16(column),
                saturate_u16(pane.rect.y.saturating_add(screen_row))
            ),
            SetForegroundColor(theme.non_text)
        )
        .context("move to soft ruler")?;
        write!(out, "|").context("write soft ruler")?;
        paint::restore_base(out, theme)?;
    }
    Ok(())
}

fn erase_row_tail(out: &mut Stdout) -> Result<()> {
    execute!(out, Clear(ClearType::UntilNewLine)).context("erase row tail")?;
    Ok(())
}

fn draw_filler_rows(out: &mut Stdout, pane: &PaneFrame<'_>, theme: &Theme) -> Result<()> {
    let height = pane.rect.height.saturating_sub(1);
    for screen_row in pane.lines.len()..height {
        execute!(
            out,
            MoveTo(
                saturate_u16(pane.rect.x),
                saturate_u16(pane.rect.y.saturating_add(screen_row))
            ),
            SetForegroundColor(theme.non_text)
        )
        .context("move to filler row")?;
        if pane.signs.is_some() {
            write!(out, " ").context("write empty git sign")?;
        }
        write!(out, "~").context("write filler")?;
        paint::restore_base(out, theme)?;
        erase_row_tail(out)?;
    }
    Ok(())
}

fn gutter_width(pane: &PaneFrame<'_>) -> usize {
    let numbers = pane.gutter.first().map_or(0, |line| line.chars().count());
    numbers + usize::from(pane.signs.is_some())
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

fn write_padded(out: &mut Stdout, text: &str, width: usize) -> Result<()> {
    let text = truncate_to_width(text, width);
    let padding = width.saturating_sub(text.chars().count());
    write!(out, "{text}{}", " ".repeat(padding)).context("write padded row")?;
    Ok(())
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
    Syntax(SyntaxHighlight),
    Diagnostic(crate::frame::DiagnosticSeverity),
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
                Tint::Syntax(span.highlight),
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

fn draw_highlight(out: &mut Stdout, pane: &PaneFrame<'_>, theme: &Theme) -> Result<()> {
    let gutter_width = gutter_width(pane);
    let view = SpanView {
        window: pane.lines,
        top: pane.viewport.top(),
        viewport: &pane.viewport,
        gutter: gutter_width,
        leftcol: pane.viewport.leftcol(),
        tabstop: pane.tabstop,
    };
    for span in syntax_highlight_spans(pane.syntax_spans, &view) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    for span in search_highlight_spans(pane.search_spans, &view) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    for span in preview_highlight_spans(pane.preview_spans, &view) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    for span in highlight_spans(
        &view,
        pane.selection,
        visual_cursor_cell(pane, gutter_width),
    ) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    for span in diagnostic_highlight_spans(pane.diagnostic_spans, &view) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    Ok(())
}

fn diagnostic_highlight_spans(spans: &[DiagnosticSpan], view: &SpanView) -> Vec<HighlightSpan> {
    spans
        .iter()
        .filter_map(|span| {
            span_segment_at(
                span.row,
                span.start,
                span.end,
                view,
                Tint::Diagnostic(span.severity),
            )
        })
        .collect()
}

fn visual_cursor_cell(pane: &PaneFrame<'_>, gutter_width: usize) -> Option<(usize, usize)> {
    pane.selection.as_ref()?;
    Some((pane.row, pane.cursor_col.saturating_sub(gutter_width)))
}

fn paint_span(
    out: &mut Stdout,
    pane: &PaneFrame<'_>,
    span: &HighlightSpan,
    gutter_width: usize,
    theme: &Theme,
) -> Result<()> {
    move_to_segment(
        out,
        pane.rect
            .x
            .saturating_add(span.from)
            .saturating_add(gutter_width),
        pane.rect.y.saturating_add(span.screen_row),
    )?;
    match span.tint {
        Tint::Selection => paint_tinted(out, &span.text, &theme.selection, theme),
        Tint::SearchMatch => paint_tinted(out, &span.text, &theme.search_match, theme),
        Tint::SearchCurrent => paint_tinted(out, &span.text, &theme.search_current, theme),
        Tint::Substitute => paint_tinted(out, &span.text, &theme.substitute, theme),
        Tint::Syntax(highlight) => paint_syntax(out, &span.text, highlight, theme),
        Tint::Diagnostic(severity) => paint_underlined(out, &span.text, severity, theme),
    }
}

fn paint_underlined(
    out: &mut Stdout,
    text: &str,
    severity: crate::frame::DiagnosticSeverity,
    theme: &Theme,
) -> Result<()> {
    paint::apply_underline(out, theme.diagnostic_color(severity))?;
    write_segment(out, text)?;
    paint::clear_underline(out)
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

fn draw_completion(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let Some(menu) = frame.completion.as_ref() else {
        return Ok(());
    };
    if menu.items.is_empty() {
        return Ok(());
    }
    let Some(pane) = frame.panes.iter().find(|pane| pane.active) else {
        return Ok(());
    };
    let height = pane.rect.height.saturating_sub(1);
    let shown = menu.items.len().min(height);
    if shown == 0 {
        return Ok(());
    }
    let start = menu_start(shown, menu.selected);
    for offset in 0..shown {
        let index = start + offset;
        let row = pane
            .rect
            .y
            .saturating_add(height.saturating_sub(shown))
            .saturating_add(offset);
        execute!(out, MoveTo(saturate_u16(pane.rect.x), saturate_u16(row)))
            .context("move to completion row")?;
        if menu.selected == Some(index) {
            paint::apply_match(out, &frame.theme.selection)?;
        }
        write_padded(out, &menu.items[index], pane.rect.width)?;
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

fn draw_overlay(out: &mut Stdout, frame: &Frame) -> Result<()> {
    let Some(overlay) = frame.overlay.as_ref() else {
        return Ok(());
    };
    if let Some(list) = overlay.list.as_ref() {
        draw_overlay_list(out, frame, list)?;
    }
    if let Some(panel) = overlay.panel.as_ref() {
        draw_overlay_panel(out, frame, panel)?;
    }
    if let Some(menu) = overlay.leader.as_ref() {
        leader::draw(out, frame, menu)?;
    }
    Ok(())
}

fn draw_overlay_list(
    out: &mut Stdout,
    frame: &Frame,
    list: &crate::frame::OverlayList<'_>,
) -> Result<()> {
    clear_overlay_rect(out, list.rect, frame.theme)?;
    draw_rounded_box(out, list.rect, frame.theme)?;
    let width = list.rect.width.saturating_sub(2);
    let height = list.rect.height.saturating_sub(2);
    if width == 0 || height == 0 {
        return Ok(());
    }
    let start = menu_start(height, list.selected);
    for offset in 0..height {
        let index = start.saturating_add(offset);
        let row = list.rect.y.saturating_add(offset).saturating_add(1);
        execute!(
            out,
            MoveTo(
                saturate_u16(list.rect.x.saturating_add(1)),
                saturate_u16(row)
            )
        )
        .context("move to overlay item")?;
        if list.selected == Some(index) {
            paint::apply_match(out, &frame.theme.selection)?;
        } else {
            paint::apply_colors(
                out,
                frame.theme.base.foreground,
                frame.theme.float_background,
                "apply overlay item",
            )?;
        }
        let item = list.items.get(index).map_or("", String::as_str);
        write_padded(out, item, width)?;
        paint::restore_base(out, frame.theme)?;
    }
    Ok(())
}

fn draw_overlay_panel(
    out: &mut Stdout,
    frame: &Frame,
    panel: &crate::frame::OverlayPanel<'_>,
) -> Result<()> {
    clear_overlay_rect(out, panel.rect, frame.theme)?;
    draw_rounded_box(out, panel.rect, frame.theme)?;
    let width = overlay_content_width(panel.rect.width);
    let height = panel.rect.height.saturating_sub(PANEL_BORDER);
    if width == 0 || height == 0 {
        return Ok(());
    }
    for offset in 0..height {
        draw_overlay_row(out, frame, panel, offset)?;
    }
    draw_scrollbar(out, frame.theme, panel)?;
    Ok(())
}

fn draw_overlay_row(
    out: &mut Stdout,
    frame: &Frame,
    panel: &crate::frame::OverlayPanel<'_>,
    offset: usize,
) -> Result<()> {
    let width = overlay_content_width(panel.rect.width);
    let index = panel.scroll.saturating_add(offset);
    let row = panel.rect.y.saturating_add(offset).saturating_add(1);
    let x = panel
        .rect
        .x
        .saturating_add(PANEL_BORDER / 2)
        .saturating_add(PANEL_PADDING);
    execute!(out, MoveTo(saturate_u16(x), saturate_u16(row)))
        .context("move to overlay panel row")?;
    let (used, background) = match panel.lines.get(index) {
        Some(line) => (
            draw_markdown_line(out, line, width, frame.theme)?,
            overlay_background(line, frame.theme),
        ),
        None => (0, frame.theme.float_background),
    };
    if used < width {
        paint::apply_colors(
            out,
            frame.theme.base.foreground,
            background,
            "apply overlay padding",
        )?;
        write!(out, "{}", " ".repeat(width.saturating_sub(used)))
            .context("pad overlay panel row")?;
    }
    paint::restore_base(out, frame.theme)
}

fn overlay_content_width(panel_width: usize) -> usize {
    panel_width
        .saturating_sub(PANEL_BORDER)
        .saturating_sub(2 * PANEL_PADDING)
}

fn draw_scrollbar(
    out: &mut Stdout,
    theme: &Theme,
    panel: &crate::frame::OverlayPanel<'_>,
) -> Result<()> {
    let visible = panel.rect.height.saturating_sub(PANEL_BORDER);
    let Some(thumb) = scrollbar_thumb(panel.scroll, panel.lines.len(), visible) else {
        return Ok(());
    };
    paint::apply_colors(
        out,
        theme.non_text,
        theme.float_background,
        "apply overlay scrollbar",
    )?;
    let x = saturate_u16(panel.rect.right().saturating_sub(1));
    for offset in 0..visible {
        let row = panel.rect.y.saturating_add(offset).saturating_add(1);
        execute!(out, MoveTo(x, saturate_u16(row))).context("move to overlay scrollbar")?;
        let glyph = if thumb.contains(&offset) {
            '█'
        } else {
            '│'
        };
        write!(out, "{glyph}").context("write overlay scrollbar")?;
    }
    paint::restore_base(out, theme)?;
    Ok(())
}

fn scrollbar_thumb(scroll: usize, total: usize, visible: usize) -> Option<std::ops::Range<usize>> {
    if visible == 0 || total <= visible {
        return None;
    }
    let thumb = visible
        .saturating_mul(visible)
        .checked_div(total)
        .unwrap_or(0)
        .max(1)
        .min(visible);
    let travel = visible.saturating_sub(thumb);
    let max_scroll = total.saturating_sub(visible);
    let start = scroll
        .min(max_scroll)
        .saturating_mul(travel)
        .checked_div(max_scroll)
        .unwrap_or(0);
    Some(start..start.saturating_add(thumb))
}

fn overlay_background(line: &crate::markdown::MarkdownLine, theme: &Theme) -> Color {
    if line.code {
        theme.code_background
    } else {
        theme.float_background
    }
}

fn draw_markdown_line(
    out: &mut Stdout,
    line: &crate::markdown::MarkdownLine,
    width: usize,
    theme: &Theme,
) -> Result<usize> {
    let mut used = 0;
    let background = overlay_background(line, theme);
    for span in &line.spans {
        if used >= width {
            break;
        }
        let text = truncate_to_display_width(&span.text, width.saturating_sub(used));
        if text.is_empty() {
            continue;
        }
        apply_markdown_style(out, span.style, background, theme)?;
        write!(out, "{text}").context("write overlay span")?;
        used = used.saturating_add(display_width(text));
    }
    execute!(out, SetAttribute(Attribute::Reset)).context("reset overlay style")?;
    paint::apply_base(out, theme)?;
    Ok(used)
}

fn apply_markdown_style(
    out: &mut Stdout,
    style: crate::markdown::MarkdownStyle,
    background: Color,
    theme: &Theme,
) -> Result<()> {
    execute!(out, SetAttribute(Attribute::Reset)).context("reset overlay style")?;
    let style = markdown_style(style, theme).unwrap_or_default();
    let background = style.background.unwrap_or(background);
    paint::apply_colors(
        out,
        style.foreground.unwrap_or(theme.base.foreground),
        background,
        "apply overlay background",
    )?;
    apply_attributes(out, style.bold, style.italic, style.underline)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
struct MarkdownRenderStyle {
    foreground: Option<Color>,
    background: Option<Color>,
    bold: bool,
    italic: bool,
    underline: bool,
}

fn markdown_style(
    style: crate::markdown::MarkdownStyle,
    theme: &Theme,
) -> Option<MarkdownRenderStyle> {
    use crate::markdown::MarkdownStyle;
    let syntax = |name: &str| theme.syntax.named(name).and_then(|style| style.foreground);
    match style {
        MarkdownStyle::Normal => None,
        MarkdownStyle::Strong | MarkdownStyle::Heading => Some(MarkdownRenderStyle {
            foreground: syntax("keyword"),
            bold: true,
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Emphasis => Some(MarkdownRenderStyle {
            foreground: syntax("function"),
            italic: true,
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Code => Some(MarkdownRenderStyle {
            foreground: syntax("string"),
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Quote => Some(MarkdownRenderStyle {
            foreground: syntax("comment"),
            ..MarkdownRenderStyle::default()
        }),
        MarkdownStyle::Syntax(highlight) => Some(theme.syntax.style(highlight).map_or_else(
            MarkdownRenderStyle::default,
            |style| MarkdownRenderStyle {
                foreground: style.foreground,
                background: style.background,
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
            },
        )),
    }
}

fn paint_syntax(
    out: &mut Stdout,
    text: &str,
    highlight: SyntaxHighlight,
    theme: &Theme,
) -> Result<()> {
    let style = theme.syntax.style(highlight).unwrap_or_default();
    execute!(out, SetAttribute(Attribute::Reset)).context("reset syntax style")?;
    paint::apply_colors(
        out,
        style.foreground.unwrap_or(theme.base.foreground),
        style.background.unwrap_or(theme.base.background),
        "apply syntax colors",
    )?;
    apply_attributes(out, style.bold, style.italic, style.underline)?;
    write_segment(out, text)?;
    execute!(out, SetAttribute(Attribute::Reset)).context("reset syntax attributes")?;
    paint::restore_base(out, theme)
}

fn apply_attributes(out: &mut Stdout, bold: bool, italic: bool, underline: bool) -> Result<()> {
    if bold {
        execute!(out, SetAttribute(Attribute::Bold)).context("apply bold")?;
    }
    if italic {
        execute!(out, SetAttribute(Attribute::Italic)).context("apply italic")?;
    }
    if underline {
        execute!(out, SetAttribute(Attribute::Underlined)).context("apply underline")?;
    }
    Ok(())
}

fn clear_overlay_rect(out: &mut Stdout, rect: crate::frame::Rect, theme: &Theme) -> Result<()> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    paint::apply_colors(
        out,
        theme.base.foreground,
        theme.float_background,
        "clear overlay surface",
    )?;
    for row in rect.y..rect.bottom() {
        execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(row)))
            .context("move to overlay row")?;
        write_padded(out, "", rect.width)?;
    }
    Ok(())
}

fn draw_rounded_box(out: &mut Stdout, rect: crate::frame::Rect, theme: &Theme) -> Result<()> {
    if rect.width < 2 || rect.height < 2 {
        return Ok(());
    }
    execute!(out, SetForegroundColor(theme.non_text)).context("apply overlay border color")?;
    let inner = rect.width.saturating_sub(2);
    let top = format!("╭{}╮", "─".repeat(inner));
    let bottom = format!("╰{}╯", "─".repeat(inner));
    execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(rect.y)))
        .context("move to overlay border")?;
    write!(out, "{top}").context("write overlay top border")?;
    execute!(
        out,
        MoveTo(
            saturate_u16(rect.x),
            saturate_u16(rect.bottom().saturating_sub(1))
        )
    )
    .context("move to overlay bottom border")?;
    write!(out, "{bottom}").context("write overlay bottom border")?;
    for row in rect.y.saturating_add(1)..rect.bottom().saturating_sub(1) {
        execute!(out, MoveTo(saturate_u16(rect.x), saturate_u16(row)))
            .context("move to overlay left border")?;
        write!(out, "│").context("write overlay left border")?;
        execute!(
            out,
            MoveTo(
                saturate_u16(rect.right().saturating_sub(1)),
                saturate_u16(row)
            )
        )
        .context("move to overlay right border")?;
        write!(out, "│").context("write overlay right border")?;
    }
    paint::restore_base(out, theme)?;
    Ok(())
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
    let layout = layout_message_row_with_prompt(
        frame.prompt.as_ref(),
        MessageParts {
            command: frame.command,
            search: frame.search,
            delimiter: frame.search_delimiter,
            message: frame.message,
            search_status: frame.search_status,
        },
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

struct MessageParts<'a> {
    command: Option<&'a str>,
    search: Option<&'a str>,
    delimiter: char,
    message: Option<&'a str>,
    search_status: Option<&'a str>,
}

#[cfg(test)]
fn layout_message_row(
    command: Option<&str>,
    search: Option<&str>,
    search_delimiter: char,
    message: Option<&str>,
    search_status: Option<&str>,
    showcmd: Option<&str>,
    width: usize,
) -> MessageLayout {
    layout_message_row_with_prompt(
        None,
        MessageParts {
            command,
            search,
            delimiter: search_delimiter,
            message,
            search_status,
        },
        showcmd,
        width,
    )
}

fn layout_message_row_with_prompt(
    prompt: Option<&PromptMenu<'_>>,
    parts: MessageParts<'_>,
    showcmd: Option<&str>,
    width: usize,
) -> MessageLayout {
    let show = showcmd.unwrap_or("");
    let left = message_left_with_prompt(prompt, &parts);
    MessageLayout {
        line: truncate_to_width(&left, left_width(width, show)),
        showcmd: truncate_to_width(show, width),
        showcmd_col: showcmd_column(show, width),
    }
}

fn message_left_with_prompt(prompt: Option<&PromptMenu<'_>>, parts: &MessageParts<'_>) -> String {
    if let Some(prompt) = prompt {
        return format!("{}: {}", prompt.label, prompt.value);
    }
    if let Some(text) = parts.command {
        return format!(":{text}");
    }
    if let Some(text) = parts.search {
        return format!("{}{text}", parts.delimiter);
    }
    parts
        .message
        .or(parts.search_status)
        .unwrap_or("")
        .to_string()
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
    if let Some(col) = prompt_cursor_col_with_prompt(
        frame.prompt.as_ref(),
        frame.command,
        frame.search,
        frame.viewport.width(),
        frame.showcmd.as_deref(),
    ) {
        return place_prompt_cursor(out, col, frame.viewport);
    }
    let Some(pane) = frame.panes.iter().find(|pane| pane.active) else {
        return Ok(());
    };
    let height = pane.rect.height.saturating_sub(1);
    if pane.row < pane.viewport.top() || pane.row >= pane.viewport.top().saturating_add(height) {
        return Ok(());
    }
    let x = pane
        .rect
        .x
        .saturating_add(pane.cursor_col.min(pane.rect.width.saturating_sub(1)));
    let y = pane
        .rect
        .y
        .saturating_add(pane.row.saturating_sub(pane.viewport.top()));
    execute!(out, MoveTo(saturate_u16(x), saturate_u16(y))).context("place cursor")?;
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
    prompt_cursor_col_with_prompt(None, command, search, width, showcmd)
}

fn prompt_cursor_col_with_prompt(
    prompt: Option<&PromptMenu<'_>>,
    command: Option<&str>,
    search: Option<&str>,
    width: usize,
    showcmd: Option<&str>,
) -> Option<usize> {
    if let Some(prompt) = prompt {
        let prefix = char_len(prompt.label).saturating_add(2);
        return Some(
            prefix
                .saturating_add(prompt.cursor)
                .min(prompt_edge(width, showcmd)),
        );
    }
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
    fn layout_prefers_prompt_over_command() {
        let prompt = PromptMenu {
            label: "rename",
            value: "value",
            cursor: 5,
        };
        let layout = layout_message_row_with_prompt(
            Some(&prompt),
            MessageParts {
                command: Some("w"),
                search: None,
                delimiter: '/',
                message: Some("saved"),
                search_status: None,
            },
            None,
            20,
        );
        assert_eq!(layout.line, "rename: value");
    }

    #[test]
    fn prompt_cursor_follows_the_prompt_value() {
        let prompt = PromptMenu {
            label: "rename",
            value: "value",
            cursor: 5,
        };
        assert_eq!(
            prompt_cursor_col_with_prompt(Some(&prompt), Some("w"), None, 30, None),
            Some(13)
        );
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
    fn diagnostic_spans_clip_to_visible_text() {
        let lines = lines_of("abcdef");
        let viewport = viewport();
        let view = span_view(&lines, &viewport);
        let diagnostic = DiagnosticSpan {
            row: 0,
            start: 2,
            end: 4,
            severity: crate::frame::DiagnosticSeverity::Error,
        };
        let spans = diagnostic_highlight_spans(&[diagnostic], &view);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].screen_row, 0);
        assert_eq!(spans[0].from, 2);
        assert_eq!(spans[0].text, "cd");
        assert_eq!(
            spans[0].tint,
            Tint::Diagnostic(crate::frame::DiagnosticSeverity::Error)
        );
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

    fn highlight_named(name: &str) -> SyntaxHighlight {
        kawi_syntax::registry()
            .index_of(name)
            .expect("registered capture")
    }

    #[test]
    fn syntax_spans_use_group_tints() {
        let lines = lines_of("fn main");
        let viewport = Viewport::new(10, 20);
        let view = span_view(&lines, &viewport);
        let keyword = highlight_named("keyword");
        let function = highlight_named("function");
        let spans = vec![
            SyntaxSpan {
                row: 0,
                start: 0,
                end: 2,
                highlight: keyword,
            },
            SyntaxSpan {
                row: 0,
                start: 3,
                end: 7,
                highlight: function,
            },
        ];
        let painted = syntax_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 2);
        assert_eq!(painted[0].tint, Tint::Syntax(keyword));
        assert_eq!(painted[0].text, "fn");
        assert_eq!(painted[1].tint, Tint::Syntax(function));
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
            highlight: highlight_named("string"),
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
            highlight: highlight_named("variable"),
        }];
        let painted = syntax_highlight_spans(&spans, &view);
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].text, "        a");
    }

    #[test]
    fn syntax_attributes_reach_overlay_style() {
        use crate::theme::{SyntaxStyle, SyntaxStyles};
        use std::collections::BTreeMap;
        let mut theme = Theme::default();
        let mut styles = BTreeMap::new();
        for name in [
            "comment",
            "keyword",
            "string",
            "number",
            "constant",
            "function",
            "type",
            "variable",
            "property",
            "operator",
            "punctuation",
            "attribute",
            "tag",
            "invalid",
        ] {
            let color = theme
                .syntax
                .named(name)
                .and_then(|style| style.foreground)
                .expect("base color");
            styles.insert(name.to_string(), SyntaxStyle::with_foreground(color));
        }
        styles.insert(
            "comment".to_string(),
            SyntaxStyle {
                background: Some(Color::Blue),
                bold: true,
                italic: true,
                underline: true,
                ..theme.syntax.named("comment").expect("comment")
            },
        );
        theme.syntax = SyntaxStyles::from_keyed(styles);
        let style = markdown_style(
            crate::markdown::MarkdownStyle::Syntax(highlight_named("comment")),
            &theme,
        )
        .expect("syntax style");
        assert_eq!(style.background, Some(Color::Blue));
        assert!(style.bold);
        assert!(style.italic);
        assert!(style.underline);
    }

    #[test]
    fn theme_change_recolors_overlay_syntax() {
        let default = Theme::named("default").expect("default theme");
        let gruvbox = Theme::named("one-dark").expect("one dark theme");
        let style = crate::markdown::MarkdownStyle::Syntax(highlight_named("keyword"));
        let default_color = markdown_style(style, &default)
            .and_then(|style| style.foreground)
            .expect("default color");
        let gruvbox_color = markdown_style(style, &gruvbox)
            .and_then(|style| style.foreground)
            .expect("gruvbox color");
        assert_ne!(default_color, gruvbox_color);
    }

    #[test]
    fn code_lines_use_the_code_background() {
        let theme = Theme::named("default").expect("default theme");
        let code = crate::markdown::MarkdownLine::code(Vec::new());
        let text = crate::markdown::MarkdownLine::blank();
        assert_eq!(overlay_background(&code, &theme), theme.code_background);
        assert_eq!(overlay_background(&text, &theme), theme.float_background);
    }

    #[test]
    fn scrollbar_thumb_tracks_the_visible_range() {
        assert_eq!(scrollbar_thumb(0, 10, 10), None);
        assert_eq!(scrollbar_thumb(0, 100, 10), Some(0..1));
        assert_eq!(scrollbar_thumb(45, 100, 10), Some(4..5));
        assert_eq!(scrollbar_thumb(90, 100, 10), Some(9..10));
        assert_eq!(scrollbar_thumb(90, 11, 10), Some(1..10));
    }
}
