use super::*;

#[test]
fn hidden_cell_offset_matches_the_cursor_cell() {
    let span = HighlightSpan {
        screen_row: 2,
        from: 4,
        text: "TODO".to_string(),
        tint: Tint::Plugin(plugin_style(true)),
    };
    assert_eq!(hidden_cell_offset(&span, (2, 4)), Some(0));
    assert_eq!(hidden_cell_offset(&span, (2, 7)), Some(3));
    assert_eq!(hidden_cell_offset(&span, (2, 8)), None);
    assert_eq!(hidden_cell_offset(&span, (1, 4)), None);
}

#[test]
fn selection_syntax_paints_syntax_foreground_and_selection_background() {
    let theme = Theme::default();
    let keyword = highlight_named("keyword");
    let span = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: "fn".to_string(),
        tint: Tint::SelectionSyntax(keyword),
    };
    let mut out = surface(2, 1, &theme);
    paint_span_at(&mut out, &span, 0, 0, &theme, theme.base.background).unwrap();
    let style = out.cell(0, 0).expect("syntax cell").style();
    assert_eq!(
        style.foreground(),
        theme
            .syntax
            .named("keyword")
            .expect("keyword")
            .foreground
            .expect("foreground")
    );
    assert_eq!(style.background(), theme.selection.background);
    assert_eq!(out.cell(1, 0).expect("syntax cell").text(), "n");
}
