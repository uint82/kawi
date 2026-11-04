use anyhow::{Context, Result};
use crossterm::style::Color;
use std::borrow::Cow;
use std::io::Write;

use crate::display;
use crate::frame::{DecorationSpan, DiagnosticUnderline, PaneFrame, PluginStyle};
use crate::paint;
use crate::surface::Surface;
use crate::text::display_width;
use crate::theme::{MatchColors, Theme};

use super::spans::{
    highlight_spans, preview_highlight_spans, search_highlight_spans, span_segment_at,
    syntax_highlight_spans, HighlightSpan, SpanView, Tint,
};
use crate::render::bracket;
use crate::render::diagnostic;
use crate::render::furniture::gutter_width;
use crate::render::selection;
use crate::render::style::{apply_attributes, paint_syntax};

pub(crate) fn draw_highlight(out: &mut Surface, pane: &PaneFrame<'_>, theme: &Theme) -> Result<()> {
    let gutter_width = gutter_width(pane);
    let view = SpanView {
        window: pane.lines,
        rows: Cow::Borrowed(pane.rows),
        viewport: &pane.viewport,
        gutter: gutter_width,
        leftcol: pane.viewport.leftcol(),
        tabstop: pane.tabstop,
    };
    let syntax_spans = syntax_highlight_spans(pane.syntax_spans, &view);
    for span in &syntax_spans {
        paint_span(out, pane, span, gutter_width, theme)?;
    }
    {
        let mut painter = DecorationPainter {
            out,
            pane,
            gutter_width,
            theme,
        };
        let cursor = cursor_text_cell(pane, gutter_width);
        for span in decoration_highlight_spans(pane.decorations, &view) {
            painter.paint(&span, cursor)?;
        }
    }
    for span in search_highlight_spans(pane.search_spans, &view) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    for span in preview_highlight_spans(pane.preview_spans, &view) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    let selection_spans = selection::split_by_syntax(
        highlight_spans(
            &view,
            pane.selection,
            visual_cursor_cell(pane, gutter_width),
        ),
        &syntax_spans,
        pane.tabstop,
    );
    for span in &selection_spans {
        paint_span(out, pane, span, gutter_width, theme)?;
    }
    paint::restore_base(out, theme);
    for span in bracket::bracket_match_spans(pane.bracket_match, &view) {
        paint_span(out, pane, &span, gutter_width, theme)?;
    }
    for span in diagnostic::diagnostic_highlight_spans(pane.diagnostic_spans, &view) {
        diagnostic::paint_span(
            out,
            pane,
            &span,
            gutter_width,
            theme,
            &selection_spans,
            &syntax_spans,
        )?;
    }
    Ok(())
}

fn decoration_highlight_spans(spans: &[DecorationSpan], view: &SpanView) -> Vec<HighlightSpan> {
    spans
        .iter()
        .filter_map(|span| {
            span_segment_at(
                span.row,
                span.start,
                span.end,
                view,
                Tint::Plugin(span.style),
            )
        })
        .collect()
}

fn visual_cursor_cell(pane: &PaneFrame<'_>, gutter_width: usize) -> Option<(usize, usize)> {
    pane.selection.as_ref()?;
    Some((pane.row, pane.cursor_col.saturating_sub(gutter_width)))
}

fn cursor_text_cell(pane: &PaneFrame<'_>, gutter_width: usize) -> Option<(usize, usize)> {
    Some((
        pane.cursor_screen_row?,
        pane.cursor_col.checked_sub(gutter_width)?,
    ))
}

pub(crate) fn hidden_cell_offset(span: &HighlightSpan, cursor: (usize, usize)) -> Option<usize> {
    if span.screen_row != cursor.0 {
        return None;
    }
    let offset = cursor.1.checked_sub(span.from)?;
    (offset < display_width(&span.text)).then_some(offset)
}

struct DecorationPainter<'a, 'b> {
    out: &'a mut Surface,
    pane: &'a PaneFrame<'b>,
    gutter_width: usize,
    theme: &'a Theme,
}

impl DecorationPainter<'_, '_> {
    fn paint(&mut self, span: &HighlightSpan, cursor: Option<(usize, usize)>) -> Result<()> {
        paint_span(self.out, self.pane, span, self.gutter_width, self.theme)?;
        let Some(offset) = cursor.and_then(|cursor| hidden_cell_offset(span, cursor)) else {
            return Ok(());
        };
        let Tint::Plugin(style) = span.tint else {
            return Ok(());
        };
        if !style.hidden {
            return Ok(());
        }
        let text = display::slice_screen(&span.text, offset, offset + 1, self.pane.tabstop);
        if text.is_empty() {
            return Ok(());
        }
        let cell = HighlightSpan {
            screen_row: span.screen_row,
            from: span.from + offset,
            text,
            tint: Tint::Plugin(PluginStyle {
                hidden: false,
                ..style
            }),
        };
        paint_span(self.out, self.pane, &cell, self.gutter_width, self.theme)
    }
}

fn paint_span(
    out: &mut Surface,
    pane: &PaneFrame<'_>,
    span: &HighlightSpan,
    gutter_width: usize,
    theme: &Theme,
) -> Result<()> {
    paint_span_at(
        out,
        span,
        pane.rect.x.saturating_add(gutter_width),
        pane.rect.y,
        theme,
        theme.base.background,
    )
}

pub(crate) fn paint_span_at(
    out: &mut Surface,
    span: &HighlightSpan,
    x: usize,
    y: usize,
    theme: &Theme,
    background: Color,
) -> Result<()> {
    move_to_segment(
        out,
        x.saturating_add(span.from),
        y.saturating_add(span.screen_row),
    )?;
    match span.tint {
        Tint::Selection => selection::paint_plain(out, &span.text, theme, background),
        Tint::SelectionSyntax(highlight) => paint_syntax(
            out,
            &span.text,
            highlight,
            theme,
            theme.selection.background,
        ),
        Tint::BracketMatch => bracket::paint_bracket_match(out, &span.text, theme),
        Tint::SearchMatch => paint_tinted(out, &span.text, &theme.search_match, theme, background),
        Tint::SearchCurrent => {
            paint_tinted(out, &span.text, &theme.search_current, theme, background)
        }
        Tint::Substitute => paint_tinted(out, &span.text, &theme.substitute, theme, background),
        Tint::Syntax(highlight) => paint_syntax(out, &span.text, highlight, theme, background),
        Tint::Plugin(style) => paint_plugin(out, &span.text, style, theme, background),
        Tint::Diagnostic { severity, tag } => diagnostic::paint_diagnostic(
            out,
            &span.text,
            diagnostic::DiagnosticPaint {
                severity,
                tag,
                selected: false,
                syntax: None,
                underline: DiagnosticUnderline::default(),
            },
            theme,
        ),
    }
}

pub(crate) fn paint_plugin(
    out: &mut Surface,
    text: &str,
    style: PluginStyle,
    theme: &Theme,
    background: Color,
) -> Result<()> {
    out.reset_attributes();
    paint::apply_colors(out, style.foreground, style.background);
    apply_attributes(out, style.bold, style.italic, style.underline);
    out.set_hidden(style.hidden);
    write_segment(out, text)?;
    out.reset_attributes();
    paint::apply_colors(out, theme.base.foreground, background);
    Ok(())
}

fn paint_tinted(
    out: &mut Surface,
    text: &str,
    colors: &MatchColors,
    theme: &Theme,
    background: Color,
) -> Result<()> {
    paint::apply_match(out, colors);
    write_segment(out, text)?;
    paint::apply_colors(out, theme.base.foreground, background);
    Ok(())
}

pub(crate) fn move_to_segment(out: &mut Surface, col: usize, screen_row: usize) -> Result<()> {
    out.move_to(col, screen_row);
    Ok(())
}

pub(crate) fn write_segment(out: &mut Surface, text: &str) -> Result<()> {
    write!(out, "{text}").context("write segment")?;
    Ok(())
}
