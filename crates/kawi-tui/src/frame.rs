use crate::statusbar::StatusInfo;
use crate::viewport::Viewport;

pub struct Frame<'a> {
    pub lines: &'a [String],
    pub col: usize,
    pub row: usize,
    pub viewport: &'a Viewport,
    pub status: &'a StatusInfo<'a>,
    pub message: Option<&'a str>,
    pub command: Option<&'a str>,
    pub search: Option<&'a str>,
    pub showcmd: Option<&'a str>,
    pub insert_mode: bool,
    pub selection: Option<((usize, usize), (usize, usize))>,
}
