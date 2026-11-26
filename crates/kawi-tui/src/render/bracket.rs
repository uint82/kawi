use super::highlight::{span_segment_at, HighlightSpan, SpanView, Tint};

pub(super) fn bracket_match_spans(
    matched: Option<kawi_editor::BracketMatch>,
    view: &SpanView,
) -> Vec<HighlightSpan> {
    let Some(matched) = matched else {
        return Vec::new();
    };
    let Some(first) = bracket_position_span(matched.first, view) else {
        return Vec::new();
    };
    let Some(second) = bracket_position_span(matched.second, view) else {
        return Vec::new();
    };
    vec![first, second]
}

fn bracket_position_span(position: (usize, usize), view: &SpanView) -> Option<HighlightSpan> {
    span_segment_at(
        position.0,
        position.1,
        position.1.saturating_add(1),
        view,
        Tint::BracketMatch,
    )
}
