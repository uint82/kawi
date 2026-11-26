use super::*;

use crate::frame::{DiagnosticSeverity, DiagnosticSpan, DiagnosticUnderline, SearchSpan};
use crate::render::highlight::draw_highlight;

#[test]
fn search_survives_a_diagnostic_through_draw_highlight() {
    let theme = Theme::default();
    let lines = vec!["fn".to_string()];
    let rows = vec![0];
    let diagnostic_spans = [DiagnosticSpan {
        row: 0,
        start: 0,
        end: 2,
        severity: DiagnosticSeverity::Error,
        tag: None,
    }];
    let search_spans = [SearchSpan {
        row: 0,
        start: 0,
        end: 2,
        current: false,
    }];
    let status = status();
    let pane = pane_frame(
        PaneParts {
            lines: &lines,
            rows: &rows,
            gutter: &[],
            diagnostic_spans: &diagnostic_spans,
            diagnostic_underline: DiagnosticUnderline::Curly,
            search_spans: &search_spans,
            ..PaneParts::default()
        },
        &status,
    );
    let mut out = surface(16, 2, &theme);
    out.write_text("fn");
    draw_highlight(&mut out, &pane, &theme);
    let style = out.cell(0, 0).expect("cell").style();
    assert_eq!(style.foreground(), theme.search_match.foreground);
    assert_eq!(style.background(), theme.search_match.background);
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}
