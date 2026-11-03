use crate::frame::{DecorationStyle, Rect};

pub struct PluginPanel<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub status: &'a str,
    pub focused: bool,
    pub lines: &'a [PluginPanelLine],
    pub offset: usize,
}

pub struct PluginPanelLine {
    pub text: String,
    pub style: Option<DecorationStyle>,
}
