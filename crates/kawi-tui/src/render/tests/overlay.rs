use super::*;
use crate::frame::CompletionLine;
use crate::render::completion_row;
use crate::render::overlay::{overlay_background, scrollbar_thumb};
use kawi_completion::CompletionKind;

#[test]
fn code_lines_use_the_code_background() {
    let theme = Theme::named("default").expect("default theme");
    let code = crate::markdown::MarkdownLine::code(Vec::new());
    let text = crate::markdown::MarkdownLine::blank();
    assert_eq!(overlay_background(&code, &theme), theme.code_background);
    assert_eq!(overlay_background(&text, &theme), theme.float_background);
}

#[test]
fn scrollbar_thumb_tracks_the_visible_range() {
    assert_eq!(scrollbar_thumb(0, 10, 10), None);
    assert_eq!(scrollbar_thumb(0, 100, 10), Some(0..1));
    assert_eq!(scrollbar_thumb(45, 100, 10), Some(4..5));
    assert_eq!(scrollbar_thumb(90, 100, 10), Some(9..10));
    assert_eq!(scrollbar_thumb(90, 11, 10), Some(1..10));
}

fn row_text(surface: &Surface, width: usize) -> String {
    (0..width)
        .map(|x| surface.cell(x, 0).expect("cell").text())
        .collect()
}

fn completion_line(label: &str, kind: Option<CompletionKind>) -> CompletionLine {
    CompletionLine {
        label: label.to_string(),
        detail: Some("fn".to_string()),
        kind,
        deprecated: false,
        highlights: Vec::new(),
    }
}

#[test]
fn completion_rows_right_align_the_kind() {
    let theme = Theme::default();
    let mut out = Surface::new(20, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(
        &mut out,
        &theme,
        &completion_line("print", Some(CompletionKind::Function)),
        20,
        8,
        false,
    )
    .expect("draw row");
    assert_eq!(row_text(&out, 20), "print  fn   function");
    assert_eq!(
        out.cell(12, 0).expect("kind cell").style().foreground(),
        theme.completion.color(CompletionKind::Function)
    );
    assert_eq!(
        out.cell(7, 0).expect("detail cell").style().foreground(),
        theme.non_text
    );
}

#[test]
fn selected_completion_rows_use_the_selection_colors() {
    let theme = Theme::default();
    let mut out = Surface::new(20, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(
        &mut out,
        &theme,
        &completion_line("print", Some(CompletionKind::Function)),
        20,
        8,
        true,
    )
    .expect("draw row");
    for column in [0, 7, 12, 19] {
        let style = out.cell(column, 0).expect("cell").style();
        assert_eq!(style.foreground(), theme.selection.foreground);
        assert_eq!(style.background(), theme.selection.background);
    }
}

#[test]
fn completion_rows_without_a_kind_leave_the_column_blank() {
    let theme = Theme::default();
    let mut out = Surface::new(16, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(
        &mut out,
        &theme,
        &completion_line("value", None),
        16,
        8,
        false,
    )
    .expect("draw row");
    assert_eq!(row_text(&out, 16), "value           ");
}

#[test]
fn deprecated_completion_rows_are_crossed_out() {
    let theme = Theme::default();
    let mut line = completion_line("value", Some(CompletionKind::Function));
    line.deprecated = true;
    let mut out = Surface::new(16, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(&mut out, &theme, &line, 16, 8, false).expect("draw row");
    assert!(out.cell(0, 0).expect("cell").style().is_crossed_out());
    assert!(!out.cell(6, 0).expect("cell").style().is_crossed_out());
    assert!(!out.cell(8, 0).expect("cell").style().is_crossed_out());
}

#[test]
fn matched_label_characters_use_the_match_color() {
    let theme = Theme::default();
    let mut line = completion_line("print", None);
    line.detail = None;
    line.highlights = vec![0..1, 3..5];
    let mut out = Surface::new(8, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(&mut out, &theme, &line, 8, 0, false).expect("draw row");
    let color = |column| out.cell(column, 0).expect("cell").style().foreground();
    assert_eq!(color(0), theme.completion.matched());
    assert_eq!(color(1), theme.base.foreground);
    assert_eq!(color(3), theme.completion.matched());
    assert_eq!(color(4), theme.completion.matched());
    assert_eq!(color(5), theme.base.foreground);
}

#[test]
fn highlight_ranges_survive_multibyte_characters() {
    let theme = Theme::default();
    let mut line = completion_line("école", None);
    line.detail = None;
    line.highlights = vec![0..2, 3..5];
    let mut out = Surface::new(4, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(&mut out, &theme, &line, 4, 0, false).expect("draw row");
    assert_eq!(row_text(&out, 4), "écol");
    assert_eq!(
        out.cell(0, 0).expect("cell").style().foreground(),
        theme.completion.matched()
    );
    assert_eq!(
        out.cell(1, 0).expect("cell").style().foreground(),
        theme.base.foreground
    );
    assert_eq!(
        out.cell(2, 0).expect("cell").style().foreground(),
        theme.completion.matched()
    );
}

#[test]
fn highlight_ranges_survive_wide_characters() {
    let theme = Theme::default();
    let mut line = completion_line("日本x", None);
    line.detail = None;
    line.highlights = vec![0..3, 6..7];
    let mut out = Surface::new(5, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(&mut out, &theme, &line, 5, 0, false).expect("draw row");
    assert_eq!(row_text(&out, 5), "日本x");
    assert_eq!(
        out.cell(0, 0).expect("cell").style().foreground(),
        theme.completion.matched()
    );
    assert_eq!(
        out.cell(2, 0).expect("cell").style().foreground(),
        theme.base.foreground
    );
    assert_eq!(
        out.cell(4, 0).expect("cell").style().foreground(),
        theme.completion.matched()
    );
}

#[test]
fn highlights_past_the_truncated_label_are_ignored() {
    let theme = Theme::default();
    let mut line = completion_line("value", None);
    line.detail = None;
    line.highlights = std::iter::once(3..5).collect();
    let mut out = Surface::new(2, 1, theme.base.foreground, theme.base.background);
    completion_row::draw_row(&mut out, &theme, &line, 2, 0, false).expect("draw row");
    assert_eq!(row_text(&out, 2), "va");
    for column in 0..2 {
        assert_eq!(
            out.cell(column, 0).expect("cell").style().foreground(),
            theme.base.foreground
        );
    }
}
