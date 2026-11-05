use super::*;

use crate::frame::{DiagnosticUnderline, PaneFrame, Rect};
use crate::render::highlight::draw_highlight;
use crate::render::line::draw_visible_lines;
use crate::statusbar::{ModeKind, StatusInfo};
use kawi_syntax::SyntaxSpan;

fn status() -> StatusInfo<'static> {
    StatusInfo {
        filename: "test",
        dirty: false,
        readonly: false,
        row: 0,
        col: 0,
        virtcol: 0,
        line_empty: false,
        empty_buffer: false,
        mode: "NORMAL",
        mode_kind: ModeKind::Normal,
        lsp: None,
        debug: None,
        diagnostics: None,
        diagnostics_severity: None,
        workspace_diagnostics: None,
        workspace_diagnostics_severity: None,
        external: None,
        git: None,
        filetype: None,
        indent: None,
        encoding: None,
        eol: None,
        selection: None,
        macro_recording: None,
        percent: None,
        plugins: &[],
    }
}

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
    let viewport = Viewport::new(2, 16);
    let status = status();
    let pane = PaneFrame {
        rect: Rect::new(0, 0, 16, 2),
        active: true,
        lines: &lines,
        rows: &rows,
        gutter: &gutter,
        signs: None,
        diagnostic_spans: &[],
        diagnostic_underline: DiagnosticUnderline::default(),
        decorations: &[],
        virtual_texts: &[],
        plugin_virtual_texts: &[],
        left_width: 0,
        search_spans: &[],
        preview_spans: &[],
        syntax_spans: &syntax_spans,
        rulers: &[],
        selection: None,
        bracket_match: None,
        viewport,
        cursor_col: 0,
        cursor_screen_row: None,
        row: 0,
        tabstop: 8,
        status: &status,
    };
    let mut out = Surface::new(16, 2, theme.base.foreground, theme.base.background);
    draw_visible_lines(&mut out, &pane, &theme).expect("draw lines");
    draw_highlight(&mut out, &pane, &theme).expect("draw highlight");
    let placeholder = out.cell(2, 0).expect("placeholder cell").style();
    assert_eq!(placeholder.foreground(), theme.special_key);
    assert!(placeholder.is_italic());
}
