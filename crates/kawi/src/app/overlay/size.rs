use kawi_tui::frame::Rect;
use kawi_tui::markdown::MarkdownLine;
use kawi_tui::overlay::{PANEL_BORDER, PANEL_PADDING};
use kawi_tui::text::display_width;

const PANEL_CHROME: usize = PANEL_BORDER + 2 * PANEL_PADDING;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PanelLimits {
    pub min_width: usize,
    pub max_width: usize,
    pub max_height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SizedPanel {
    pub width: usize,
    pub height: usize,
    pub lines: Vec<MarkdownLine>,
}

pub(super) fn size_panel(
    limits: PanelLimits,
    render: impl FnOnce(usize) -> Vec<MarkdownLine>,
) -> SizedPanel {
    let max_width = limits.max_width.max(1);
    let min_width = limits.min_width.min(max_width).max(1);
    let content_width = max_width.saturating_sub(PANEL_CHROME).max(1);
    let mut lines = render(content_width);
    clean_lines(&mut lines);
    let widest = lines.iter().map(line_width).max().unwrap_or(0);
    let width = widest
        .saturating_add(PANEL_CHROME)
        .clamp(min_width, max_width);
    let height = lines.len().saturating_add(2).min(limits.max_height.max(1));
    SizedPanel {
        width,
        height,
        lines,
    }
}

fn clean_lines(lines: &mut Vec<MarkdownLine>) {
    let Some(first) = lines.iter().position(|line| !is_blank(line)) else {
        lines.clear();
        return;
    };
    let last = lines
        .iter()
        .rposition(|line| !is_blank(line))
        .unwrap_or(first);
    let mut cleaned = Vec::with_capacity(last.saturating_sub(first).saturating_add(1));
    for line in lines.drain(first..=last) {
        if is_blank(&line) && cleaned.last().is_some_and(is_blank) {
            continue;
        }
        cleaned.push(line);
    }
    *lines = cleaned;
}

fn line_width(line: &MarkdownLine) -> usize {
    line.spans
        .iter()
        .map(|span| display_width(&span.text))
        .sum()
}

fn is_blank(line: &MarkdownLine) -> bool {
    line.spans.iter().all(|span| span.text.trim().is_empty())
}

pub(super) fn cursor_vertical_capacity(area: Rect, anchor: (usize, usize)) -> usize {
    let below = area.bottom().saturating_sub(anchor.1.saturating_add(1));
    let above = anchor.1.saturating_sub(area.y);
    below.max(above)
}

pub(super) fn side_vertical_capacity(area: Rect, top: usize) -> usize {
    area.bottom().saturating_sub(top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_tui::markdown::{self, MarkdownLine, MarkdownSpan, MarkdownStyle};

    fn line(text: &str) -> MarkdownLine {
        MarkdownLine {
            spans: vec![MarkdownSpan {
                text: text.to_string(),
                style: MarkdownStyle::Normal,
                link: None,
            }],
            code: false,
        }
    }

    fn limits(min_width: usize, max_width: usize, max_height: usize) -> PanelLimits {
        PanelLimits {
            min_width,
            max_width,
            max_height,
        }
    }

    fn panel(text: &str, limits: PanelLimits) -> SizedPanel {
        size_panel(limits, |width| {
            markdown::render_spans(
                &[MarkdownSpan {
                    text: text.to_string(),
                    style: MarkdownStyle::Normal,
                    link: None,
                }],
                width,
            )
        })
    }

    #[test]
    fn one_line_uses_the_min_width() {
        let panel = panel("short", limits(20, 64, 24));
        assert_eq!(panel.width, 20);
        assert_eq!(panel.height, 3);
    }

    #[test]
    fn long_lines_wrap_at_the_max_width() {
        let panel = panel(&"x".repeat(60), limits(20, 24, 24));
        assert_eq!(panel.width, 24);
        assert!(panel
            .lines
            .iter()
            .all(|line| display_width(&line.text()) <= 20));
    }

    #[test]
    fn width_includes_border_and_padding_columns() {
        let panel = panel(&"x".repeat(10), limits(8, 64, 24));
        assert_eq!(panel.width, 14);
    }

    #[test]
    fn blank_edges_and_doubles_are_removed() {
        let panel = size_panel(limits(20, 64, 24), |_| {
            vec![
                MarkdownLine::blank(),
                line("first"),
                MarkdownLine::blank(),
                MarkdownLine::blank(),
                line("second"),
                MarkdownLine::blank(),
            ]
        });
        assert_eq!(panel.lines.len(), 3);
        assert_eq!(panel.height, 5);
        assert_eq!(panel.lines[1].text(), "");
    }

    #[test]
    fn wide_characters_and_tabs_use_display_width() {
        let panel = panel("中\t文", limits(20, 64, 24));
        assert_eq!(panel.width, 20);
        assert_eq!(panel.lines[0].text(), "中      文");
    }

    #[test]
    fn height_is_capped() {
        let panel = size_panel(limits(20, 64, 5), |_| {
            (0..10).map(|index| line(&index.to_string())).collect()
        });
        assert_eq!(panel.height, 5);
        assert_eq!(panel.lines.len(), 10);
    }

    #[test]
    fn max_width_below_min_width_clamps_to_the_window() {
        let panel = panel(&"x".repeat(60), limits(20, 8, 24));
        assert_eq!(panel.width, 8);
    }

    #[test]
    fn cursor_capacity_uses_the_larger_side() {
        let area = Rect::new(0, 0, 80, 30);
        assert_eq!(cursor_vertical_capacity(area, (10, 5)), 24);
        assert_eq!(cursor_vertical_capacity(area, (10, 25)), 25);
    }

    #[test]
    fn side_capacity_starts_at_the_list() {
        let area = Rect::new(0, 0, 80, 30);
        assert_eq!(side_vertical_capacity(area, 12), 18);
        assert_eq!(side_vertical_capacity(area, 40), 0);
    }
}
