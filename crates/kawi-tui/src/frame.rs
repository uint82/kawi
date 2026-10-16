use crate::statusbar::StatusInfo;
use crate::theme::Theme;
use kawi_editor::Viewport;

pub struct Frame<'a> {
    pub lines: &'a [String],
    pub col: usize,
    pub row: usize,
    pub viewport: &'a Viewport,
    pub status: &'a StatusInfo<'a>,
    pub message: Option<&'a str>,
    pub command: Option<&'a str>,
    pub search: Option<&'a str>,
    pub search_status: Option<&'a str>,
    pub search_spans: &'a [SearchSpan],
    pub showcmd: Option<&'a str>,
    pub insert_mode: bool,
    pub selection: Option<((usize, usize), (usize, usize))>,
    pub theme: &'a Theme,
}

pub struct SearchSpan {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub current: bool,
}
