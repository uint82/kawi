use std::borrow::Cow;

use kawi_buffer::unicode;
use kawi_editor::Viewport;
use kawi_syntax::{SyntaxHighlight, SyntaxSpan};

use crate::display;
use crate::frame::{PluginStyle, SearchSpan};

mod selection;

pub use selection::highlight_spans;

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
