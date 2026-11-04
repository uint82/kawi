use std::borrow::Cow;

use kawi_buffer::unicode;
use kawi_editor::Viewport;
use kawi_syntax::{SyntaxHighlight, SyntaxSpan};

use crate::display;
use crate::frame::{PluginStyle, SearchSpan, Selection};
use crate::text::char_count;

pub struct HighlightSpan {
    pub screen_row: usize,
    pub from: usize,
    pub text: String,
    pub tint: Tint,
}

pub struct SpanView<'a> {
    pub window: &'a [String],
    pub rows: Cow<'a, [usize]>,
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
    SelectionSyntax(SyntaxHighlight),
    BracketMatch,
    SearchMatch,
    SearchCurrent,
    Substitute,
    Syntax(SyntaxHighlight),
    Plugin(PluginStyle),
    Diagnostic {
        severity: crate::frame::DiagnosticSeverity,
        tag: Option<crate::frame::DiagnosticTag>,
    },
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
        let Some(buffer_row) = view.rows.get(screen_row).copied() else {
            continue;
        };
        if buffer_row < block_top || buffer_row > block_bottom {
            continue;
        }
        let Some(text) = view.window.get(screen_row) else {
            continue;
        };
        let to = if ragged {
            char_count(text)
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
    let Some(buffer_row) = view.rows.get(screen_row).copied() else {
        return;
    };
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
    let line_len = char_count(text);
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

pub(crate) fn preview_highlight_spans(spans: &[SearchSpan], view: &SpanView) -> Vec<HighlightSpan> {
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

pub(crate) fn span_segment_at(
    row: usize,
    start: usize,
    end: usize,
    view: &SpanView,
    tint: Tint,
) -> Option<HighlightSpan> {
    let screen_row = view.rows.binary_search(&row).ok()?;
    let text = view.window.get(screen_row)?;
    if screen_row >= view.viewport.height() {
        return None;
    }
    let start = unicode::cluster_start(text, start);
    let end = unicode::snap_end(text, end);
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
