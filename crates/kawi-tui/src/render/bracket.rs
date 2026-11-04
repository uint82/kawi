use anyhow::Result;

use crate::paint;
use crate::surface::Surface;
use crate::theme::Theme;

use super::highlight::{span_segment_at, write_segment, HighlightSpan, SpanView, Tint};

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

pub(super) fn paint_bracket_match(out: &mut Surface, text: &str, theme: &Theme) -> Result<()> {
    paint::apply_match(out, &theme.bracket_match);
    out.set_bold(true);
    write_segment(out, text)?;
    out.reset_attributes();
    paint::restore_base(out, theme);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_match_does_not_leave_underline_attributes() {
        let theme = Theme::default();
        let mut out = Surface::new(1, 1, theme.base.foreground, theme.base.background);
        paint_bracket_match(&mut out, "(", &theme).expect("paint");
        let style = out.cell(0, 0).expect("bracket cell").style();
        assert_eq!(out.cell(0, 0).expect("bracket cell").text(), "(");
        assert!(style.is_bold());
        assert!(style.underline().is_none());
        assert_eq!(style.foreground(), theme.bracket_match.foreground);
        assert_eq!(style.background(), theme.bracket_match.background);
    }
}
