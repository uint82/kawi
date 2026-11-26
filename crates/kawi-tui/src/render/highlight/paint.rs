use std::borrow::Cow;

use crate::display;
use crate::frame::{DecorationSpan, PaneFrame, PluginStyle};
use crate::surface::Surface;
use crate::text::display_width;
use crate::theme::Theme;

use super::patch::{patch_span_at, patch_spans_at, plugin_patch, PatchTarget};
use super::spans::{
    highlight_spans, preview_highlight_spans, search_highlight_spans, span_segment_at,
    syntax_highlight_spans, HighlightSpan, SpanView, Tint,
};
use crate::render::bracket;
use crate::render::diagnostic;
use crate::render::furniture::gutter_width;
use crate::render::special_key::{draw_special_keys, SpecialKeyDraw};

pub(crate) fn draw_highlight(out: &mut Surface, pane: &PaneFrame<'_>, theme: &Theme) {
    let gutter_width = gutter_width(pane);
    let view = SpanView {
        window: pane.lines,
        rows: Cow::Borrowed(pane.rows),
        viewport: &pane.viewport,
        gutter: gutter_width,
        leftcol: pane.viewport.leftcol(),
        tabstop: pane.tabstop,
    };
    let target = PatchTarget {
        x: pane.rect.x.saturating_add(gutter_width),
        y: pane.rect.y,
        theme,
    };
    let diagnostic_spans = diagnostic::diagnostic_highlight_spans(pane.diagnostic_spans, &view);
    let diagnostic_layer = diagnostic::DiagnosticLayer {
        spans: &diagnostic_spans,
        target,
        underline: pane.diagnostic_underline,
    };
    diagnostic::patch_tag_foreground(out, &diagnostic_layer);
    let syntax_spans = syntax_highlight_spans(pane.syntax_spans, &view);
    patch_spans_at(out, &syntax_spans, target);
    draw_special_keys(out, SpecialKeyDraw::from_pane(pane, theme));
    paint_decorations(out, pane, &view, target);
    patch_spans_at(
        out,
        &search_highlight_spans(pane.search_spans, &view),
        target,
    );
    patch_spans_at(
        out,
        &preview_highlight_spans(pane.preview_spans, &view),
        target,
    );
    let selection_spans = highlight_spans(
        &view,
        pane.selection,
        visual_cursor_cell(pane, gutter_width),
    );
    patch_spans_at(out, &selection_spans, target);
    patch_spans_at(
        out,
        &bracket::bracket_match_spans(pane.bracket_match, &view),
        target,
    );
    diagnostic::patch_decorations(out, &diagnostic_layer);
}

fn paint_decorations(
    out: &mut Surface,
    pane: &PaneFrame<'_>,
    view: &SpanView<'_>,
    target: PatchTarget<'_>,
) {
    let mut painter = DecorationPainter { out, pane, target };
    for span in decoration_highlight_spans(pane.decorations, view) {
        painter.paint(&span);
    }
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
    target: PatchTarget<'a>,
}

impl DecorationPainter<'_, '_> {
    fn paint(&mut self, span: &HighlightSpan) {
        patch_span_at(self.out, span, self.target);
        self.reveal_hidden(span);
    }

    fn reveal_hidden(&mut self, span: &HighlightSpan) {
        let gutter_width = self.target.x.saturating_sub(self.pane.rect.x);
        let Some(offset) = cursor_text_cell(self.pane, gutter_width)
            .and_then(|cursor| hidden_cell_offset(span, cursor))
        else {
            return;
        };
        let Tint::Plugin(style) = span.tint else {
            return;
        };
        if !style.hidden {
            return;
        }
        let text = display::slice_screen(&span.text, offset, offset + 1, self.pane.tabstop);
        if text.is_empty() {
            return;
        }
        self.out.patch_range(
            self.target
                .x
                .saturating_add(span.from)
                .saturating_add(offset),
            self.target.y.saturating_add(span.screen_row),
            display_width(&text),
            plugin_patch(PluginStyle {
                hidden: false,
                ..style
            }),
        );
    }
}
