use super::*;
use crate::render::highlight::patch::{patch_span_at, PatchTarget};
use crate::surface::StylePatch;

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
fn syntax_patch_keeps_the_selection_background() {
    let theme = Theme::default();
    let keyword = highlight_named("keyword");
    let span = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: "fn".to_string(),
        tint: Tint::Syntax(keyword),
    };
    let mut out = surface(2, 1, &theme);
    out.write_text("fn");
    out.patch_range(
        0,
        0,
        2,
        StylePatch {
            background: Some(theme.selection.background),
            ..StylePatch::default()
        },
    );
    patch_span_at(
        &mut out,
        &span,
        PatchTarget {
            x: 0,
            y: 0,
            theme: &theme,
        },
    );
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

#[test]
fn selection_patch_only_changes_the_background() {
    let theme = Theme::default();
    let span = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: "x".to_string(),
        tint: Tint::Selection,
    };
    let mut out = surface(1, 1, &theme);
    out.write_text("x");
    out.patch_range(
        0,
        0,
        1,
        StylePatch {
            foreground: Some(theme.search_match.foreground),
            ..StylePatch::default()
        },
    );
    patch_span_at(
        &mut out,
        &span,
        PatchTarget {
            x: 0,
            y: 0,
            theme: &theme,
        },
    );
    let style = out.cell(0, 0).expect("selection cell").style();
    assert_eq!(style.foreground(), theme.search_match.foreground);
    assert_eq!(style.background(), theme.selection.background);
}
