use anyhow::{Context, Result};
use crossterm::{
    execute,
    style::{Attribute, SetAttribute},
};
use std::io::Stdout;

use crate::paint;
use crate::theme::Theme;

use super::{span_segment_at, write_segment, HighlightSpan, SpanView, Tint};

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

pub(super) fn paint_bracket_match(out: &mut Stdout, text: &str, theme: &Theme) -> Result<()> {
    paint::apply_match(out, &theme.bracket_match)?;
    execute!(out, SetAttribute(Attribute::Bold)).context("apply bracket match weight")?;
    write_segment(out, text)?;
    execute!(out, SetAttribute(Attribute::NoBold)).context("clear bracket match weight")?;
    paint::restore_base(out, theme)
}
