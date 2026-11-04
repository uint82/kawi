use super::*;

#[test]
fn bracket_match_spans_convert_tabs_wide_and_multibyte_cells() {
    let lines = lines_of("\té界(x)");
    let viewport = Viewport::new(10, 40);
    let view = span_view(&lines, &viewport);
    let spans = bracket_match_spans(
        Some(kawi_editor::BracketMatch {
            first: (0, 3),
            second: (0, 5),
        }),
        &view,
    );
    assert_eq!(spans.len(), 2);
    assert_eq!((spans[0].from, spans[0].text.as_str()), (11, "("));
    assert_eq!((spans[1].from, spans[1].text.as_str()), (13, ")"));
    assert!(spans.iter().all(|span| span.tint == Tint::BracketMatch));
}

#[test]
fn bracket_match_spans_require_both_brackets_on_screen() {
    let lines = lines_of("(\n)");
    let viewport = Viewport::new(1, 20);
    let view = span_view(&lines, &viewport);
    assert!(bracket_match_spans(
        Some(kawi_editor::BracketMatch {
            first: (0, 0),
            second: (1, 0),
        }),
        &view,
    )
    .is_empty());
    let lines = lines_of(&format!("({})", " ".repeat(30)));
    let viewport = Viewport::new(10, 10);
    let view = span_view(&lines, &viewport);
    assert!(bracket_match_spans(
        Some(kawi_editor::BracketMatch {
            first: (0, 0),
            second: (0, 31),
        }),
        &view,
    )
    .is_empty());
}

#[test]
fn bracket_match_spans_compose_with_selection_and_cursor() {
    let lines = lines_of("a (b) c");
    let viewport = Viewport::new(10, 20);
    let view = span_view(&lines, &viewport);
    let selection = highlight_spans(
        &view,
        Some(Selection::Range {
            start: (0, 3),
            end: (0, 3),
        }),
        Some((0, 2)),
    );
    let brackets = bracket_match_spans(
        Some(kawi_editor::BracketMatch {
            first: (0, 2),
            second: (0, 4),
        }),
        &view,
    );
    assert_eq!(selection.len(), 1);
    assert_eq!(selection[0].text, "b");
    assert_eq!(brackets.len(), 2);
    assert_eq!((brackets[0].from, brackets[0].text.as_str()), (2, "("));
    assert_eq!((brackets[1].from, brackets[1].text.as_str()), (4, ")"));
}

#[test]
fn search_spans_tint_current() {
    let lines = lines_of("ar ar");
    let spans = vec![
        SearchSpan {
            row: 0,
            start: 0,
            end: 2,
            current: true,
        },
        SearchSpan {
            row: 0,
            start: 3,
            end: 5,
            current: false,
        },
    ];
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let painted = search_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 2);
    assert_eq!(painted[0].tint, Tint::SearchCurrent);
    assert_eq!(painted[0].text, "ar");
    assert_eq!(painted[1].tint, Tint::SearchMatch);
    assert_eq!(painted[1].from, 3);
}

#[test]
fn search_spans_skip_outside_window() {
    let lines = lines_of("ar\nar");
    let spans = vec![
        SearchSpan {
            row: 0,
            start: 0,
            end: 2,
            current: false,
        },
        SearchSpan {
            row: 5,
            start: 0,
            end: 2,
            current: false,
        },
        SearchSpan {
            row: 1,
            start: 2,
            end: 2,
            current: false,
        },
    ];
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let painted = search_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].screen_row, 0);
}

#[test]
fn preview_spans_use_substitute_tint() {
    let lines = lines_of("orange");
    let spans = vec![SearchSpan {
        row: 0,
        start: 0,
        end: 6,
        current: true,
    }];
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let painted = preview_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].tint, Tint::Substitute);
    assert_eq!(painted[0].text, "orange");
}

#[test]
fn spans_shift_by_leftcol() {
    let lines = lines_of("0123456789");
    let spans = vec![SearchSpan {
        row: 0,
        start: 0,
        end: 10,
        current: false,
    }];
    let viewport = viewport();
    let mut view = span_view(&lines, &viewport);
    view.leftcol = 3;
    let painted = search_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].from, 0);
    assert_eq!(painted[0].text, "3456789");
}

#[test]
fn spans_clip_to_text_width_with_gutter() {
    let lines = lines_of("012345678901234567890123");
    let spans = vec![SearchSpan {
        row: 0,
        start: 0,
        end: 24,
        current: false,
    }];
    let viewport = viewport();
    let mut view = span_view(&lines, &viewport);
    view.gutter = 4;
    view.leftcol = 5;
    let painted = search_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].from, 0);
    assert_eq!(painted[0].text, "5678901234567890");
}

#[test]
fn spans_expand_tabs() {
    let lines = lines_of("\tab");
    let spans = vec![SearchSpan {
        row: 0,
        start: 0,
        end: 2,
        current: false,
    }];
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let painted = search_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].text, "        a");
}

#[test]
fn selection_spans_expand_tabs() {
    let lines = lines_of("a\tb");
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let painted = highlight_spans(
        &view,
        Some(Selection::Range {
            start: (0, 0),
            end: (0, 2),
        }),
        None,
    );
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].text, "a       b");
}

#[test]
fn syntax_spans_use_group_tints() {
    let lines = lines_of("fn main");
    let viewport = Viewport::new(10, 20);
    let view = span_view(&lines, &viewport);
    let keyword = highlight_named("keyword");
    let function = highlight_named("function");
    let spans = vec![
        SyntaxSpan {
            row: 0,
            start: 0,
            end: 2,
            highlight: keyword,
        },
        SyntaxSpan {
            row: 0,
            start: 3,
            end: 7,
            highlight: function,
        },
    ];
    let painted = syntax_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 2);
    assert_eq!(painted[0].tint, Tint::Syntax(keyword));
    assert_eq!(painted[0].text, "fn");
    assert_eq!(painted[1].tint, Tint::Syntax(function));
    assert_eq!(painted[1].from, 3);
}

#[test]
fn syntax_spans_shift_by_leftcol() {
    let lines = lines_of("0123456789");
    let viewport = Viewport::new(10, 20);
    let mut view = span_view(&lines, &viewport);
    view.leftcol = 4;
    let spans = vec![SyntaxSpan {
        row: 0,
        start: 0,
        end: 10,
        highlight: highlight_named("string"),
    }];
    let painted = syntax_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].from, 0);
    assert_eq!(painted[0].text, "456789");
}

#[test]
fn syntax_spans_expand_tabs() {
    let lines = lines_of("\tab");
    let viewport = Viewport::new(10, 20);
    let view = span_view(&lines, &viewport);
    let spans = vec![SyntaxSpan {
        row: 0,
        start: 0,
        end: 2,
        highlight: highlight_named("variable"),
    }];
    let painted = syntax_highlight_spans(&spans, &view);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].text, "        a");
}
