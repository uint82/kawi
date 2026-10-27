use anyhow::Result;
use crossterm::style::Color;
use kawi_syntax::SyntaxHighlight;
use std::io::Write;

use crate::display;
use crate::paint;
use crate::text::display_width;
use crate::theme::Theme;

use super::{write_segment, HighlightSpan, Tint};

pub(super) fn split_by_syntax(
    spans: Vec<HighlightSpan>,
    syntax_spans: &[HighlightSpan],
    tabstop: usize,
) -> Vec<HighlightSpan> {
    let mut split = Vec::new();
    for span in spans {
        split_span(&mut split, span, syntax_spans, tabstop);
    }
    split
}

fn split_span(
    out: &mut Vec<HighlightSpan>,
    span: HighlightSpan,
    syntax_spans: &[HighlightSpan],
    tabstop: usize,
) {
    let start = span.from;
    let end = start.saturating_add(display_width(&span.text));
    let ranges = syntax_ranges(syntax_spans, span.screen_row, start, end);
    if ranges.is_empty() {
        out.push(span);
        return;
    }
    for (from, to, highlight) in split_ranges(start, end, &ranges) {
        let text = display::slice_screen(&span.text, from - start, to - start, tabstop);
        if text.is_empty() {
            continue;
        }
        out.push(HighlightSpan {
            screen_row: span.screen_row,
            from,
            text,
            tint: highlight.map_or(Tint::Selection, Tint::SelectionSyntax),
        });
    }
}

fn syntax_ranges(
    spans: &[HighlightSpan],
    screen_row: usize,
    start: usize,
    end: usize,
) -> Vec<(usize, usize, SyntaxHighlight)> {
    spans
        .iter()
        .filter(|span| span.screen_row == screen_row)
        .filter_map(|span| {
            let Tint::Syntax(highlight) = span.tint else {
                return None;
            };
            let from = span.from.max(start);
            let to = span.from.saturating_add(display_width(&span.text)).min(end);
            (from < to).then_some((from, to, highlight))
        })
        .collect()
}

pub(super) fn split_ranges<T: Copy>(
    start: usize,
    end: usize,
    ranges: &[(usize, usize, T)],
) -> Vec<(usize, usize, Option<T>)> {
    let mut cuts = vec![start, end];
    for (from, to, _) in ranges {
        cuts.push((*from).max(start));
        cuts.push((*to).min(end));
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .filter_map(|pair| {
            let (from, to) = (pair[0], pair[1]);
            if from >= to {
                return None;
            }
            let value = ranges
                .iter()
                .find(|(start, end, _)| *start <= from && to <= *end)
                .map(|(_, _, value)| *value);
            Some((from, to, value))
        })
        .collect()
}

pub(super) fn paint_plain<W: Write>(
    out: &mut W,
    text: &str,
    theme: &Theme,
    background: Color,
) -> Result<()> {
    paint::apply_colors(
        out,
        theme.base.foreground,
        theme.selection.background,
        "apply selection colors",
    )?;
    write_segment(out, text)?;
    paint::apply_colors(
        out,
        theme.base.foreground,
        background,
        "restore selection colors",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(from: usize, text: &str) -> HighlightSpan {
        HighlightSpan {
            screen_row: 0,
            from,
            text: text.to_string(),
            tint: Tint::Selection,
        }
    }

    fn syntax(from: usize, text: &str, highlight: SyntaxHighlight) -> HighlightSpan {
        HighlightSpan {
            screen_row: 0,
            from,
            text: text.to_string(),
            tint: Tint::Syntax(highlight),
        }
    }

    fn highlight(name: &str) -> SyntaxHighlight {
        kawi_syntax::registry()
            .index_of(name)
            .expect("registered capture")
    }

    #[test]
    fn splits_selection_at_syntax_boundaries() {
        let keyword = highlight("keyword");
        let function = highlight("function");
        let spans = vec![selection(0, "fn main")];
        let syntax_spans = vec![syntax(0, "fn", keyword), syntax(3, "main", function)];
        let split = split_by_syntax(spans, &syntax_spans, 8);
        assert_eq!(
            split
                .iter()
                .map(|span| (span.from, span.text.as_str(), span.tint))
                .collect::<Vec<_>>(),
            [
                (0, "fn", Tint::SelectionSyntax(keyword)),
                (2, " ", Tint::Selection),
                (3, "main", Tint::SelectionSyntax(function)),
            ]
        );
    }

    #[test]
    fn keeps_selection_without_syntax() {
        let span = selection(2, "text");
        let split = split_by_syntax(vec![span], &[], 8);
        assert_eq!(split.len(), 1);
        assert_eq!(split[0].from, 2);
        assert_eq!(split[0].text, "text");
        assert_eq!(split[0].tint, Tint::Selection);
    }

    #[test]
    fn clips_syntax_to_selection_bounds() {
        let keyword = highlight("keyword");
        let function = highlight("function");
        let spans = vec![selection(2, "cde")];
        let syntax_spans = vec![syntax(1, "abc", keyword), syntax(4, "efgh", function)];
        let split = split_by_syntax(spans, &syntax_spans, 8);
        assert_eq!(
            split
                .iter()
                .map(|span| (span.from, span.text.as_str(), span.tint))
                .collect::<Vec<_>>(),
            [
                (2, "cd", Tint::SelectionSyntax(keyword)),
                (4, "e", Tint::SelectionSyntax(function)),
            ]
        );
    }

    #[test]
    fn plain_selection_uses_base_foreground_and_selection_background() {
        let theme = Theme::default();
        let mut out = Vec::new();
        paint_plain(&mut out, "x", &theme, theme.base.background).expect("paint selection");
        let output = String::from_utf8(out).expect("utf8");
        assert!(output.contains('x'));
        assert!(output.contains(&format_foreground(theme.base.foreground)));
        assert!(output.contains(&format_background(theme.selection.background)));
    }

    fn format_foreground(color: Color) -> String {
        let mut out = Vec::new();
        crossterm::execute!(out, crossterm::style::SetForegroundColor(color)).expect("sequence");
        String::from_utf8(out).expect("utf8")
    }

    fn format_background(color: Color) -> String {
        let mut out = Vec::new();
        crossterm::execute!(out, crossterm::style::SetBackgroundColor(color)).expect("sequence");
        String::from_utf8(out).expect("utf8")
    }
}
