use anyhow::{Context, Result};
use crossterm::{
    execute,
    style::{Attribute, SetAttribute},
};
use std::io::{Stdout, Write};

use crate::display;
use crate::frame::{DiagnosticSpan, PaneFrame};
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
    let Some(segments) = diagnostic_segments(span, selection_spans, syntax_spans, pane.tabstop)
    else {
        return super::paint_span(out, pane, span, gutter_width, theme);
    };
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
) -> Option<Vec<DiagnosticSegment>> {
    let start = span.from;
    let end = start.saturating_add(display_width(&span.text));
    let ranges: Vec<(usize, usize, ())> = selection_spans
        .iter()
        .filter(|selection| selection.screen_row == span.screen_row)
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
        .collect();
    if ranges.is_empty() {
        return None;
    }
    let mut segments = Vec::new();
    for (from, to, selected) in selection::split_ranges(start, end, &ranges) {
        let text = display::slice_screen(&span.text, from - start, to - start, tabstop);
        if text.is_empty() {
            continue;
        }
        if selected.is_none() {
            segments.push(DiagnosticSegment {
                from,
                text,
                selected: false,
                syntax: None,
            });
            continue;
        }
        push_selected_segments(
            &mut segments,
            span.screen_row,
            from,
            text,
            syntax_spans,
            tabstop,
        );
    }
    Some(segments)
}

fn push_selected_segments(
    segments: &mut Vec<DiagnosticSegment>,
    screen_row: usize,
    from: usize,
    text: String,
    syntax_spans: &[HighlightSpan],
    tabstop: usize,
) {
    let span = HighlightSpan {
        screen_row,
        from,
        text,
        tint: Tint::Selection,
    };
    for part in selection::split_by_syntax(vec![span], syntax_spans, tabstop) {
        let syntax = match part.tint {
            Tint::SelectionSyntax(highlight) => Some(highlight),
            _ => None,
        };
        segments.push(DiagnosticSegment {
            from: part.from,
            text: part.text,
            selected: true,
            syntax,
        });
    }
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
    pub(super) severity: crate::frame::DiagnosticSeverity,
    pub(super) tag: Option<crate::frame::DiagnosticTag>,
    pub(super) selected: bool,
    pub(super) syntax: Option<SyntaxHighlight>,
}

pub(super) fn paint_diagnostic<W: Write>(
    out: &mut W,
    text: &str,
    paint: DiagnosticPaint,
    theme: &Theme,
) -> Result<()> {
    if paint.selected {
        return paint_selected_diagnostic(out, text, paint, theme);
    }
    let color = theme.diagnostic_color(paint.severity);
    paint::apply_colors(
        out,
        theme.base.foreground,
        theme.base.background,
        "reset diagnostic colors",
    )?;
    match paint.tag {
        Some(crate::frame::DiagnosticTag::Unnecessary) => paint::apply_faded_text(out, color)?,
        Some(crate::frame::DiagnosticTag::Deprecated) => paint::apply_struck_text(out, color)?,
        None => paint::apply_underline(out, color)?,
    }
    write_segment(out, text)?;
    paint::clear_decorations(out)?;
    paint::restore_base(out, theme)
}

fn paint_selected_diagnostic<W: Write>(
    out: &mut W,
    text: &str,
    paint: DiagnosticPaint,
    theme: &Theme,
) -> Result<()> {
    match paint.syntax {
        Some(highlight) => {
            apply_syntax_style(out, highlight, theme, theme.selection.background)?;
        }
        None => paint::apply_colors(
            out,
            theme.base.foreground,
            theme.selection.background,
            "apply selected diagnostic colors",
        )?,
    }
    match paint.tag {
        Some(crate::frame::DiagnosticTag::Unnecessary) => {
            execute!(out, SetAttribute(Attribute::Dim)).context("apply faded diagnostic text")?;
        }
        Some(crate::frame::DiagnosticTag::Deprecated) => {
            execute!(out, SetAttribute(Attribute::CrossedOut))
                .context("apply struck diagnostic text")?;
        }
        None => paint::apply_underline(out, theme.diagnostic_color(paint.severity))?,
    }
    write_segment(out, text)?;
    paint::clear_decorations(out)?;
    paint::restore_base(out, theme)
}
