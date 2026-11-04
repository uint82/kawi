use super::{spans_of, viewport};
use crate::display;
use crate::frame::Selection;
use crate::render::selection::paint_plain;
use crate::surface::Surface;
use crate::theme::Theme;
use kawi_buffer::unicode;

const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";

const FLAG: &str = "\u{1f1ee}\u{1f1e9}";
const KEYCAP: &str = "1\u{fe0f}\u{20e3}";

fn cell_text(surface: &Surface, width: usize) -> Vec<String> {
    (0..width)
        .map(|x| surface.cell(x, 0).expect("cell").text().to_string())
        .collect()
}

fn selected_cells(line: &str, cursor: usize) -> Vec<String> {
    let theme = Theme::default();
    let width = display::screen_len(line, 8).saturating_add(1);
    let mut surface = Surface::new(width, 1, theme.base.foreground, theme.base.background);
    surface.write_text(line);
    let lines = vec![line.to_string()];
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (0, line.chars().count().saturating_sub(1)),
        }),
        &viewport(),
        Some((0, cursor)),
    );
    for span in spans {
        surface.move_to(span.from, 0);
        paint_plain(&mut surface, &span.text, &theme, theme.selection.background)
            .expect("paint selection");
    }
    cell_text(&surface, width)
}

#[test]
fn visual_painting_keeps_wide_text_at_its_cells() {
    let line = format!("{FAMILY} {FLAG} {KEYCAP} e\u{301}");
    let theme = Theme::default();
    let width = display::screen_len(&line, 8).saturating_add(1);
    let mut base = Surface::new(width, 1, theme.base.foreground, theme.base.background);
    base.write_text(&line);
    let expected = cell_text(&base, width);
    for cursor in 0..display::screen_len(&line, 8) {
        assert_eq!(selected_cells(&line, cursor), expected, "cursor {cursor}");
    }
}

#[test]
fn visual_painting_keeps_complex_emoji_aligned_for_all_ranges() {
    let lines = [
        "\u{2764}\u{fe0f} \u{2764}\u{fe0f}\u{200d}\u{1f525} \u{2764}\u{fe0f}\u{200d}\u{1fa79} \u{1f494} \u{1f495} \u{1f496} \u{1f497} \u{1f498} \u{1f49d}",
        "\u{1f3f3}\u{fe0f} \u{1f3f4} \u{1f3f3}\u{fe0f}\u{200d}\u{1f308} \u{1f3f3}\u{fe0f}\u{200d}\u{26a7}\u{fe0f} \u{1f1ee}\u{1f1e9} \u{1f1ef}\u{1f1f5} \u{1f1f0}\u{1f1f7} \u{1f1fa}\u{1f1f8} \u{1f1ec}\u{1f1e7} \u{1f1e9}\u{1f1ea} \u{1f1eb}\u{1f1f7} \u{1f1e7}\u{1f1f7} \u{1f1ee}\u{1f1f3} \u{1f1e8}\u{1f1f3}",
        "\u{1f3ae} \u{1f3a7} \u{1f3b9} \u{1f3b8} \u{1f3bb} \u{1f941} \u{1f3a8} \u{1f58c}\u{fe0f} \u{1f4da} \u{1f4bb} \u{1f5a5}\u{fe0f} \u{2328}\u{fe0f} \u{1f5b1}\u{fe0f} \u{1f4f1} \u{1f50b} \u{1f50c}",
        "\u{1f680} \u{1f6f8} \u{1f30c} \u{1f30d} \u{1f30e} \u{1f30f} \u{1f315} \u{1f311} \u{2600}\u{fe0f} \u{1f308} \u{26a1} \u{1f525} \u{1f4a7} \u{2744}\u{fe0f} \u{1f30a} \u{1f30b} \u{1f332} \u{1f344} \u{1f33b}",
    ];
    let theme = Theme::default();
    for line in lines {
        let starts: Vec<usize> = unicode::graphemes(line).map(|g| g.start()).collect();
        let width = display::screen_len(line, 8).saturating_add(1);
        let mut base = Surface::new(width, 1, theme.base.foreground, theme.base.background);
        base.write_text(line);
        let expected = cell_text(&base, width);
        let window = vec![line.to_string()];
        for &start in &starts {
            for &end in starts.iter().filter(|&&end| end >= start) {
                for &cursor_start in starts
                    .iter()
                    .filter(|&&cursor| cursor >= start && cursor <= end)
                {
                    let cursor = display::screen_col(line, cursor_start, 8);
                    let spans = spans_of(
                        &window,
                        0,
                        Some(Selection::Range {
                            start: (0, start),
                            end: (0, end),
                        }),
                        &viewport(),
                        Some((0, cursor)),
                    );
                    let mut surface =
                        Surface::new(width, 1, theme.base.foreground, theme.base.background);
                    surface.write_text(line);
                    for span in spans {
                        surface.move_to(span.from, 0);
                        paint_plain(&mut surface, &span.text, &theme, theme.selection.background)
                            .expect("paint selection");
                    }
                    assert_eq!(
                        cell_text(&surface, width),
                        expected,
                        "line {line:?} start {start} end {end} cursor {cursor_start}"
                    );
                }
            }
        }
    }
}

#[test]
fn wide_cursor_cluster_keeps_following_text_aligned() {
    let line = format!("a{FAMILY}b");
    let spans = spans_of(
        &[line],
        0,
        Some(Selection::Range {
            start: (0, 1),
            end: (0, 8),
        }),
        &viewport(),
        Some((0, 1)),
    );
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].from, 3);
    assert_eq!(spans[0].text, "b");
}

#[test]
fn linewise_visual_skips_the_whole_cursor_cluster() {
    let line = format!("a{FAMILY}b");
    let spans = spans_of(
        &[line],
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (0, 9),
        }),
        &viewport(),
        Some((0, 1)),
    );
    assert_eq!(spans.len(), 2);
    assert_eq!((spans[0].from, spans[0].text.as_str()), (0, "a"));
    assert_eq!((spans[1].from, spans[1].text.as_str()), (3, "b"));
}
