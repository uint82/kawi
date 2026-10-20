use crate::statusbar::StatusInfo;
use crate::theme::Theme;
use kawi_editor::Viewport;
use kawi_syntax::SyntaxSpan;

pub struct Frame<'a> {
    pub lines: &'a [String],
    pub gutter: &'a [String],
    pub pager: Option<&'a [String]>,
    pub completion: Option<CompletionMenu<'a>>,
    pub cursor_col: usize,
    pub row: usize,
    pub viewport: &'a Viewport,
    pub status: &'a StatusInfo<'a>,
    pub message: Option<&'a str>,
    pub command: Option<&'a str>,
    pub search: Option<&'a str>,
    pub search_delimiter: char,
    pub search_status: Option<&'a str>,
    pub search_spans: &'a [SearchSpan],
    pub preview_spans: &'a [SearchSpan],
    pub syntax_spans: &'a [SyntaxSpan],
    pub leftcol: usize,
    pub tabstop: usize,
    pub showcmd: Option<String>,
    pub cursor_style: CursorStyle,
    pub selection: Option<Selection>,
    pub theme: &'a Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Bar,
    Underline,
}

pub struct CompletionMenu<'a> {
    pub items: &'a [String],
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Range {
        start: (usize, usize),
        end: (usize, usize),
    },
    Block {
        top: usize,
        left: usize,
        bottom: usize,
        right: usize,
        ragged: bool,
    },
}

pub struct SearchSpan {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub current: bool,
}
