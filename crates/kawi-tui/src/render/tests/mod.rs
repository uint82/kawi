use std::borrow::Cow;

use kawi_editor::Viewport;
use kawi_syntax::{SyntaxHighlight, SyntaxSpan};

use crate::frame::{
    DiagnosticSpan, DiagnosticUnderline, PaneFrame, PluginStyle, Rect, SearchSpan, Selection,
};
use crate::render::highlight::spans::highlight_spans;
use crate::render::highlight::{HighlightSpan, SpanView, Tint};
use crate::statusbar::{ModeKind, StatusInfo};
use crate::surface::Surface;
use crate::theme::Theme;

mod diagnostic;
mod highlight;
mod layers;
mod overlay;
mod selection;
mod selection_wide;
mod special_key;
mod virtual_text;

pub(super) fn viewport() -> Viewport {
    Viewport::new(10, 20)
}

pub(super) fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

pub(super) fn plugin_style(hidden: bool) -> PluginStyle {
    PluginStyle {
        foreground: crossterm::style::Color::White,
        background: crossterm::style::Color::Black,
        accent: crossterm::style::Color::Blue,
        bold: false,
        italic: false,
        underline: false,
        hidden,
    }
}

pub(super) fn span_view<'a>(lines: &'a [String], viewport: &'a Viewport) -> SpanView<'a> {
    SpanView {
        window: lines,
        rows: Cow::Owned((0..lines.len()).collect()),
        viewport,
        gutter: 0,
        leftcol: 0,
        tabstop: 8,
    }
}

pub(super) fn spans_of(
    lines: &[String],
    top: usize,
    selection: Option<Selection>,
    viewport: &Viewport,
    cursor: Option<(usize, usize)>,
) -> Vec<HighlightSpan> {
    let view = SpanView {
        window: lines,
        rows: Cow::Owned((top..top.saturating_add(lines.len())).collect()),
        viewport,
        gutter: 0,
        leftcol: 0,
        tabstop: 8,
    };
    highlight_spans(&view, selection, cursor)
}

pub(super) fn surface(width: usize, height: usize, theme: &Theme) -> Surface {
    Surface::new(width, height, theme.base.foreground, theme.base.background)
}

pub(super) fn status() -> StatusInfo<'static> {
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

#[derive(Default)]
pub(super) struct PaneParts<'a> {
    pub(super) lines: &'a [String],
    pub(super) rows: &'a [usize],
    pub(super) gutter: &'a [String],
    pub(super) diagnostic_spans: &'a [DiagnosticSpan],
    pub(super) diagnostic_underline: DiagnosticUnderline,
    pub(super) search_spans: &'a [SearchSpan],
    pub(super) syntax_spans: &'a [SyntaxSpan],
    pub(super) selection: Option<Selection>,
}

pub(super) fn pane_frame<'a>(parts: PaneParts<'a>, status: &'a StatusInfo<'a>) -> PaneFrame<'a> {
    PaneFrame {
        rect: Rect::new(0, 0, 16, 2),
        active: true,
        lines: parts.lines,
        rows: parts.rows,
        gutter: parts.gutter,
        signs: None,
        diagnostic_spans: parts.diagnostic_spans,
        diagnostic_underline: parts.diagnostic_underline,
        decorations: &[],
        virtual_texts: &[],
        plugin_virtual_texts: &[],
        left_width: 0,
        search_spans: parts.search_spans,
        preview_spans: &[],
        syntax_spans: parts.syntax_spans,
        rulers: &[],
        selection: parts.selection,
        bracket_match: None,
        viewport: Viewport::new(2, 16),
        cursor_col: 0,
        cursor_screen_row: None,
        row: 0,
        tabstop: 8,
        status,
    }
}

pub(super) fn highlight_named(name: &str) -> SyntaxHighlight {
    kawi_syntax::registry()
        .index_of(name)
        .expect("registered capture")
}
