use anyhow::{Context, Result};
use crossterm::{
    execute,
    style::{Attribute, Color, SetAttribute},
};
use std::io::{Stdout, Write};

use crate::display;
use crate::frame::{DiagnosticSeverity, DiagnosticSpan, DiagnosticTag, PaneFrame};
use crate::paint;
use crate::text::display_width;
use crate::theme::Theme;
use kawi_syntax::SyntaxHighlight;

use super::{
    apply_syntax_style, move_to_segment, selection, span_segment_at, write_segment, HighlightSpan,
    SpanView, Tint,
};

pub(super) fn paint_span(
    out: &mut Stdout,
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
}

pub(super) fn paint_diagnostic<W: Write>(
    out: &mut W,
    text: &str,
    paint: DiagnosticPaint,
    theme: &Theme,
) -> Result<()> {
    apply_diagnostic_colors(out, paint, theme)?;
    apply_diagnostic_decoration(out, paint, theme)?;
    write_segment(out, text)?;
    paint::clear_decorations(out)?;
    paint::restore_base(out, theme)
}

fn apply_diagnostic_colors<W: Write>(
    out: &mut W,
    paint: DiagnosticPaint,
    theme: &Theme,
) -> Result<()> {
    let background = if paint.selected {
        theme.selection.background
    } else {
        theme.base.background
    };
    if let Some(highlight) = paint.syntax {
        return apply_syntax_style(out, highlight, theme, background);
    }
    paint::apply_colors(
        out,
        diagnostic_foreground(paint, theme),
        background,
        "apply diagnostic colors",
    )
}

fn diagnostic_foreground(paint: DiagnosticPaint, theme: &Theme) -> Color {
    if paint.tag.is_some() && !paint.selected {
        theme.diagnostic_color(paint.severity)
    } else {
        theme.base.foreground
    }
}

fn apply_diagnostic_decoration<W: Write>(
    out: &mut W,
    paint: DiagnosticPaint,
    theme: &Theme,
) -> Result<()> {
    match paint.tag {
        Some(DiagnosticTag::Unnecessary) => {
            execute!(out, SetAttribute(Attribute::Dim)).context("apply faded diagnostic text")?;
        }
        Some(DiagnosticTag::Deprecated) => {
            execute!(out, SetAttribute(Attribute::CrossedOut))
                .context("apply struck diagnostic text")?;
        }
        None => paint::apply_underline(out, theme.diagnostic_color(paint.severity))?,
    }
    Ok(())
}
