use crate::statusbar::StatusInfo;
use crate::theme::Theme;
use kawi_editor::Viewport;

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
    pub search_status: Option<&'a str>,
    pub search_spans: &'a [SearchSpan],
    pub showcmd: Option<String>,
    pub insert_mode: bool,
    pub selection: Option<Selection>,
    pub theme: &'a Theme,
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
