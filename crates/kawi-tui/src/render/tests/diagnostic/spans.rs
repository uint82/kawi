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
        severity: DiagnosticSeverity::Error,
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
            severity: DiagnosticSeverity::Error,
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
        severity: DiagnosticSeverity::Warning,
        tag: Some(DiagnosticTag::Deprecated),
    };
    let spans = diagnostic_highlight_spans(&[diagnostic], &view);
    assert_eq!(
        spans[0].tint,
        Tint::Diagnostic {
            severity: DiagnosticSeverity::Warning,
            tag: Some(DiagnosticTag::Deprecated),
        }
    );
}
