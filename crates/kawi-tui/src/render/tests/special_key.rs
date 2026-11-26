use super::*;

use crate::render::highlight::draw_highlight;
use crate::render::line::draw_visible_lines;
use kawi_syntax::SyntaxSpan;

#[test]
fn keeps_special_key_color_after_syntax_highlighting() {
    let theme = Theme::default();
    let lines = vec!["a\u{feff}b".to_string()];
    let rows = vec![0];
    let gutter = vec!["1".to_string()];
    let highlight = kawi_syntax::registry()
        .index_of("comment")
        .expect("registered comment capture");
    let syntax_spans = vec![SyntaxSpan {
        row: 0,
        start: 1,
        end: 4,
        highlight,
    }];
    let status = status();
    let pane = pane_frame(
        PaneParts {
            lines: &lines,
            rows: &rows,
            gutter: &gutter,
            syntax_spans: &syntax_spans,
            ..PaneParts::default()
        },
        &status,
    );
    let mut out = Surface::new(16, 2, theme.base.foreground, theme.base.background);
    draw_visible_lines(&mut out, &pane, &theme).expect("draw lines");
    draw_highlight(&mut out, &pane, &theme);
    let placeholder = out.cell(2, 0).expect("placeholder cell").style();
    assert_eq!(placeholder.foreground(), theme.special_key);
    assert!(placeholder.is_italic());
}
