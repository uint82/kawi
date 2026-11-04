use anyhow::Result;
use crossterm::style::Color;

use crate::display;
use crate::frame::{
    DiagnosticSeverity, DiagnosticSpan, DiagnosticTag, DiagnosticUnderline, PaneFrame,
};
use crate::paint;
use crate::surface::Surface;
use crate::text::display_width;
use crate::theme::Theme;
use kawi_syntax::SyntaxHighlight;

use super::highlight::{
    move_to_segment, span_segment_at, write_segment, HighlightSpan, SpanView, Tint,
};
use super::selection;
use super::style::apply_syntax_style;

pub(super) fn paint_span(
    out: &mut Surface,
    pane: &PaneFrame<'_>,
    span: &HighlightSpan,
    gutter_width: usize,
    theme: &Theme,
    selection_spans: &[HighlightSpan],
    syntax_spans: &[HighlightSpan],
) -> Result<()> {
    let Tint::Diagnostic { severity, tag } = span.tint else {
        return Ok(());
    };
    let paint = DiagnosticPaint {
        severity,
        tag,
        selected: false,
        syntax: None,
        underline: pane.diagnostic_underline,
    };
    let segments = diagnostic_segments(span, selection_spans, syntax_spans, pane.tabstop);
    let x = pane.rect.x.saturating_add(gutter_width);
    for segment in segments {
        move_to_segment(
            out,
            x.saturating_add(segment.from),
            pane.rect.y.saturating_add(span.screen_row),
        )?;
        paint_diagnostic(
            out,
            &segment.text,
            DiagnosticPaint {
                selected: segment.selected,
                syntax: segment.syntax,
                ..paint
            },
            theme,
        )?;
    }
    Ok(())
}

pub(super) struct DiagnosticSegment {
    pub(super) from: usize,
    pub(super) text: String,
    pub(super) selected: bool,
    pub(super) syntax: Option<SyntaxHighlight>,
}

pub(super) fn diagnostic_segments(
    span: &HighlightSpan,
    selection_spans: &[HighlightSpan],
    syntax_spans: &[HighlightSpan],
    tabstop: usize,
) -> Vec<DiagnosticSegment> {
    let start = span.from;
    let end = start.saturating_add(display_width(&span.text));
    let ranges = selection_ranges(span.screen_row, start, end, selection_spans);
    let mut segments = Vec::new();
    for (from, to, selected) in selection::split_ranges(start, end, &ranges) {
        let text = display::slice_screen(&span.text, from - start, to - start, tabstop);
        if text.is_empty() {
            continue;
        }
        let part = HighlightSpan {
            screen_row: span.screen_row,
            from,
            text,
            tint: Tint::Selection,
        };
        segments.extend(syntax_segments(
            part,
            syntax_spans,
            selected.is_some(),
            tabstop,
        ));
    }
    segments
}

fn selection_ranges(
    screen_row: usize,
    start: usize,
    end: usize,
    selection_spans: &[HighlightSpan],
) -> Vec<(usize, usize, ())> {
    selection_spans
        .iter()
        .filter(|selection| selection.screen_row == screen_row)
        .map(|selection| {
            (
                selection.from,
                selection
                    .from
                    .saturating_add(display_width(&selection.text)),
                (),
            )
        })
        .filter(|(from, to, _)| *from < end && *to > start)
        .collect()
}

fn syntax_segments(
    span: HighlightSpan,
    syntax_spans: &[HighlightSpan],
    selected: bool,
    tabstop: usize,
) -> Vec<DiagnosticSegment> {
    selection::split_by_syntax(vec![span], syntax_spans, tabstop)
        .into_iter()
        .map(|part| DiagnosticSegment {
            from: part.from,
            text: part.text,
            selected,
            syntax: match part.tint {
                Tint::SelectionSyntax(highlight) => Some(highlight),
                _ => None,
            },
        })
        .collect()
}

pub(super) fn diagnostic_highlight_spans(
    spans: &[DiagnosticSpan],
    view: &SpanView,
) -> Vec<HighlightSpan> {
    spans
        .iter()
        .filter_map(|span| {
            span_segment_at(
                span.row,
                span.start,
                span.end,
                view,
                Tint::Diagnostic {
                    severity: span.severity,
                    tag: span.tag,
                },
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(super) struct DiagnosticPaint {
    pub(super) severity: DiagnosticSeverity,
    pub(super) tag: Option<DiagnosticTag>,
    pub(super) selected: bool,
    pub(super) syntax: Option<SyntaxHighlight>,
    pub(super) underline: DiagnosticUnderline,
}

pub(super) fn paint_diagnostic(
    out: &mut Surface,
    text: &str,
    paint: DiagnosticPaint,
    theme: &Theme,
) -> Result<()> {
    apply_diagnostic_colors(out, paint, theme);
    apply_diagnostic_decoration(out, paint, theme);
    write_segment(out, text)?;
    paint::clear_decorations(out);
    paint::restore_base(out, theme);
    Ok(())
}

fn apply_diagnostic_colors(out: &mut Surface, paint: DiagnosticPaint, theme: &Theme) {
    let background = if paint.selected {
        theme.selection.background
    } else {
        theme.base.background
    };
    if let Some(highlight) = paint.syntax {
        apply_syntax_style(out, highlight, theme, background);
        return;
    }
    paint::apply_colors(out, diagnostic_foreground(paint, theme), background);
}

fn diagnostic_foreground(paint: DiagnosticPaint, theme: &Theme) -> Color {
    if paint.tag.is_some() && !paint.selected {
        theme.diagnostic_color(paint.severity)
    } else {
        theme.base.foreground
    }
}

fn apply_diagnostic_decoration(out: &mut Surface, paint: DiagnosticPaint, theme: &Theme) {
    match paint.tag {
        Some(DiagnosticTag::Unnecessary) => out.set_dim(true),
        Some(DiagnosticTag::Deprecated) => out.set_crossed_out(true),
        None => {
            paint::apply_underline(out, theme.diagnostic_color(paint.severity), paint.underline)
        }
    }
}
