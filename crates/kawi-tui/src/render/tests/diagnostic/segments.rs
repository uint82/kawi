use super::*;

#[test]
fn diagnostic_spans_clip_to_visible_text() {
    let lines = lines_of("abcdef");
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let diagnostic = DiagnosticSpan {
        row: 0,
        start: 2,
        end: 4,
        severity: crate::frame::DiagnosticSeverity::Error,
        tag: None,
    };
    let spans = diagnostic_highlight_spans(&[diagnostic], &view);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].screen_row, 0);
    assert_eq!(spans[0].from, 2);
    assert_eq!(spans[0].text, "cd");
    assert_eq!(
        spans[0].tint,
        Tint::Diagnostic {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
        }
    );
}

#[test]
fn diagnostic_tags_reach_the_render_tint() {
    let lines = lines_of("abcdef");
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let diagnostic = DiagnosticSpan {
        row: 0,
        start: 0,
        end: 2,
        severity: crate::frame::DiagnosticSeverity::Warning,
        tag: Some(crate::frame::DiagnosticTag::Deprecated),
    };
    let spans = diagnostic_highlight_spans(&[diagnostic], &view);
    assert_eq!(
        spans[0].tint,
        Tint::Diagnostic {
            severity: crate::frame::DiagnosticSeverity::Warning,
            tag: Some(crate::frame::DiagnosticTag::Deprecated),
        }
    );
}

#[test]
fn diagnostic_segments_split_selected_ranges() {
    let diagnostic = HighlightSpan {
        screen_row: 0,
        from: 2,
        text: "cdef".to_string(),
        tint: Tint::Diagnostic {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
        },
    };
    let selection = HighlightSpan {
        screen_row: 0,
        from: 3,
        text: "de".to_string(),
        tint: Tint::Selection,
    };
    let segments = diagnostic_segments(&diagnostic, &[selection], &[], 8);
    assert_eq!(
        segments
            .iter()
            .map(|segment| (segment.from, segment.text.as_str(), segment.selected))
            .collect::<Vec<_>>(),
        [(2, "c", false), (3, "de", true), (5, "f", false)]
    );
}

#[test]
fn diagnostic_segments_clip_selection_to_the_diagnostic() {
    let diagnostic = HighlightSpan {
        screen_row: 0,
        from: 2,
        text: "cd".to_string(),
        tint: Tint::Diagnostic {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
        },
    };
    let selection = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: "abcdef".to_string(),
        tint: Tint::Selection,
    };
    let segments = diagnostic_segments(&diagnostic, &[selection], &[], 8);
    assert_eq!(
        segments
            .iter()
            .map(|segment| (segment.from, segment.text.as_str(), segment.selected))
            .collect::<Vec<_>>(),
        [(2, "cd", true)]
    );
}

#[test]
fn diagnostic_segments_without_selection_keep_the_span() {
    let diagnostic = HighlightSpan {
        screen_row: 0,
        from: 2,
        text: "cd".to_string(),
        tint: Tint::Diagnostic {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
        },
    };
    let segments = diagnostic_segments(&diagnostic, &[], &[], 8);
    assert_eq!(
        segments
            .iter()
            .map(|segment| (segment.from, segment.text.as_str(), segment.selected))
            .collect::<Vec<_>>(),
        [(2, "cd", false)]
    );
}

#[test]
fn selected_diagnostics_split_by_syntax() {
    let diagnostic = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: "fn main".to_string(),
        tint: Tint::Diagnostic {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
        },
    };
    let selection = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: "fn main".to_string(),
        tint: Tint::Selection,
    };
    let keyword = highlight_named("keyword");
    let function = highlight_named("function");
    let syntax = vec![
        HighlightSpan {
            screen_row: 0,
            from: 0,
            text: "fn".to_string(),
            tint: Tint::Syntax(keyword),
        },
        HighlightSpan {
            screen_row: 0,
            from: 3,
            text: "main".to_string(),
            tint: Tint::Syntax(function),
        },
    ];
    let segments = diagnostic_segments(&diagnostic, &[selection], &syntax, 8);
    assert_eq!(
        segments
            .iter()
            .map(|segment| {
                (
                    segment.from,
                    segment.text.as_str(),
                    segment.selected,
                    segment.syntax,
                )
            })
            .collect::<Vec<_>>(),
        [
            (0, "fn", true, Some(keyword)),
            (2, " ", true, None),
            (3, "main", true, Some(function)),
        ]
    );
}

#[test]
fn unselected_diagnostics_split_by_syntax() {
    let diagnostic = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: "fn main".to_string(),
        tint: Tint::Diagnostic {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
        },
    };
    let keyword = highlight_named("keyword");
    let function = highlight_named("function");
    let syntax = vec![
        HighlightSpan {
            screen_row: 0,
            from: 0,
            text: "fn".to_string(),
            tint: Tint::Syntax(keyword),
        },
        HighlightSpan {
            screen_row: 0,
            from: 3,
            text: "main".to_string(),
            tint: Tint::Syntax(function),
        },
    ];
    let segments = diagnostic_segments(&diagnostic, &[], &syntax, 8);
    assert_eq!(
        segments
            .iter()
            .map(|segment| {
                (
                    segment.from,
                    segment.text.as_str(),
                    segment.selected,
                    segment.syntax,
                )
            })
            .collect::<Vec<_>>(),
        [
            (0, "fn", false, Some(keyword)),
            (2, " ", false, None),
            (3, "main", false, Some(function)),
        ]
    );
}
