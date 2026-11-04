use std::borrow::Cow;

use kawi_editor::Viewport;
use kawi_syntax::SyntaxHighlight;

use crate::frame::{PluginStyle, Selection};
use crate::render::highlight::spans::highlight_spans;
use crate::render::highlight::{HighlightSpan, SpanView, Tint};
use crate::surface::Surface;
use crate::theme::Theme;

mod diagnostic;
mod highlight;
mod overlay;
mod selection;
mod selection_wide;
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

pub(super) fn highlight_named(name: &str) -> SyntaxHighlight {
    kawi_syntax::registry()
        .index_of(name)
        .expect("registered capture")
}
