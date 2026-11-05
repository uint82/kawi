use crossterm::style::Color;

use kawi_buffer::unicode;

use crate::frame::PaneFrame;
use crate::surface::Surface;
use crate::theme::Theme;

use super::furniture::gutter_width;

pub(super) struct SpecialKeyArea {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) leftcol: usize,
    pub(super) tabstop: usize,
}

pub(super) struct SpecialKeyDraw<'a> {
    lines: &'a [String],
    area: SpecialKeyArea,
    color: Color,
}

impl<'a> SpecialKeyDraw<'a> {
    pub(super) fn from_pane(pane: &'a PaneFrame<'_>, theme: &Theme) -> Self {
        let gutter = gutter_width(pane);
        Self::from_area(
            pane.lines,
            SpecialKeyArea {
                x: pane.rect.x.saturating_add(gutter),
                y: pane.rect.y,
                width: pane.rect.width.saturating_sub(gutter),
                height: pane.rect.height.saturating_sub(1),
                leftcol: pane.viewport.leftcol(),
                tabstop: pane.tabstop,
            },
            theme.special_key,
        )
    }

    pub(super) fn from_area(lines: &'a [String], area: SpecialKeyArea, color: Color) -> Self {
        Self { lines, area, color }
    }

    fn draw_line(&self, out: &mut Surface, row: usize, text: &str) {
        if text.is_ascii() {
            return;
        }
        let right = self.area.leftcol.saturating_add(self.area.width);
        let y = self.area.y.saturating_add(row);
        let mut screen = 0;
        for grapheme in unicode::graphemes(text) {
            let start = screen;
            let rendered = unicode::render_cluster(grapheme.text(), start, self.area.tabstop);
            let end = start.saturating_add(rendered.width());
            screen = end;
            if start >= right {
                break;
            }
            if !rendered.is_placeholder() || start < self.area.leftcol || end > right {
                continue;
            }
            let x = self
                .area
                .x
                .saturating_add(start.saturating_sub(self.area.leftcol));
            self.recolor(out, x, y, rendered.width());
        }
    }

    fn recolor(&self, out: &mut Surface, x: usize, y: usize, width: usize) {
        for offset in 0..width {
            out.set_foreground_at(x.saturating_add(offset), y, self.color);
        }
    }
}

pub(super) fn draw_special_keys(out: &mut Surface, draw: SpecialKeyDraw<'_>) {
    for (row, text) in draw.lines.iter().take(draw.area.height).enumerate() {
        draw.draw_line(out, row, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Color = Color::Rgb { r: 255, g: 0, b: 0 };

    fn rendered(text: &str, leftcol: usize, width: usize) -> Surface {
        let mut surface = Surface::new(width, 1, Color::White, Color::Black);
        let visible = crate::display::slice_screen(text, leftcol, leftcol + width, 8);
        surface.write_text(&visible);
        let lines = vec![text.to_string()];
        draw_special_keys(
            &mut surface,
            SpecialKeyDraw::from_area(
                &lines,
                SpecialKeyArea {
                    x: 0,
                    y: 0,
                    width,
                    height: 1,
                    leftcol,
                    tabstop: 8,
                },
                RED,
            ),
        );
        surface
    }

    fn foreground(surface: &Surface, x: usize) -> Color {
        surface.cell(x, 0).expect("cell").style().foreground()
    }

    #[test]
    fn colors_placeholder_cells() {
        let surface = rendered("a\u{feff}b", 0, 16);
        assert_eq!(foreground(&surface, 0), Color::White);
        assert_eq!(surface.cell(1, 0).expect("cell").text(), "<");
        assert_eq!(surface.cell(6, 0).expect("cell").text(), ">");
        for x in 1..7 {
            assert_eq!(foreground(&surface, x), RED);
        }
        assert_eq!(foreground(&surface, 7), Color::White);
    }

    #[test]
    fn uses_the_screen_column_after_tabs() {
        let surface = rendered("\t\u{feff}", 0, 16);
        assert_eq!(foreground(&surface, 7), Color::White);
        for x in 8..14 {
            assert_eq!(foreground(&surface, x), RED);
        }
    }

    #[test]
    fn ignores_literal_placeholder_text() {
        let surface = rendered("a<feff>b", 0, 16);
        for x in 0..9 {
            assert_eq!(foreground(&surface, x), Color::White);
        }
    }

    #[test]
    fn skips_partially_visible_placeholders() {
        let left = rendered("a\u{feff}b", 2, 16);
        for x in 0..15 {
            assert_eq!(foreground(&left, x), Color::White);
        }
        let right = rendered("a\u{feff}b", 0, 4);
        for x in 0..4 {
            assert_eq!(foreground(&right, x), Color::White);
        }
    }

    #[test]
    fn preserves_context_background_and_attributes() {
        let mut surface = Surface::new(16, 1, Color::White, Color::Black);
        surface.set_background(Color::Blue);
        surface.set_bold(true);
        surface.write_text("a\u{feff}b");
        let lines = vec!["a\u{feff}b".to_string()];
        draw_special_keys(
            &mut surface,
            SpecialKeyDraw::from_area(
                &lines,
                SpecialKeyArea {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 1,
                    leftcol: 0,
                    tabstop: 8,
                },
                RED,
            ),
        );
        let style = surface.cell(1, 0).expect("cell").style();
        assert_eq!(style.foreground(), RED);
        assert_eq!(style.background(), Color::Blue);
        assert!(style.is_bold());
    }
}
