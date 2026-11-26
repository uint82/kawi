use crate::frame::{DiagnosticSeverity, DiagnosticSpan, DiagnosticTag, DiagnosticUnderline};
use crate::surface::{StylePatch, Surface, UnderlinePatch};
use crate::theme::Theme;

use super::highlight::{patch_span_at, span_segment_at, HighlightSpan, SpanView, Tint};

pub(super) struct DiagnosticLayer<'a> {
    pub(super) spans: &'a [HighlightSpan],
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) underline: DiagnosticUnderline,
}

impl DiagnosticLayer<'_> {
    fn patch(&self, out: &mut Surface, span: &HighlightSpan, patch: StylePatch) {
        patch_span_at(out, span, self.x, self.y, patch);
    }
}

pub(super) fn patch_tag_foreground(out: &mut Surface, layer: &DiagnosticLayer<'_>, theme: &Theme) {
    for span in layer.spans {
        let Tint::Diagnostic { severity, tag } = span.tint else {
            continue;
        };
        if tag.is_none() {
            continue;
        }
        layer.patch(
            out,
            span,
            StylePatch {
                foreground: Some(theme.diagnostic_color(severity)),
                ..StylePatch::default()
            },
        );
    }
}

pub(super) fn patch_decorations(out: &mut Surface, layer: &DiagnosticLayer<'_>, theme: &Theme) {
    for span in layer.spans {
        let Tint::Diagnostic { severity, tag } = span.tint else {
            continue;
        };
        layer.patch(
            out,
            span,
            decoration_patch(severity, tag, layer.underline, theme),
        );
    }
}

fn decoration_patch(
    severity: DiagnosticSeverity,
    tag: Option<DiagnosticTag>,
    underline: DiagnosticUnderline,
    theme: &Theme,
) -> StylePatch {
    match tag {
        Some(DiagnosticTag::Unnecessary) => StylePatch {
            dim: Some(true),
            ..StylePatch::default()
        },
        Some(DiagnosticTag::Deprecated) => StylePatch {
            crossed_out: Some(true),
            ..StylePatch::default()
        },
        None => StylePatch {
            underline: Some(UnderlinePatch::Diagnostic(
                underline,
                theme.diagnostic_color(severity),
            )),
            ..StylePatch::default()
        },
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
