use kawi_tui::frame::{CompletionLine, Overlay, OverlayList, OverlayPanel, Rect};
use kawi_tui::markdown::MarkdownLine;

use super::keymenu;

pub(crate) struct OverlayData<'a> {
    pub(in crate::app) list: Option<ListData<'a>>,
    pub(in crate::app) panel: Option<PanelData>,
    pub(in crate::app) key_menu: Option<keymenu::KeyMenuData>,
}

pub(crate) struct ListData<'a> {
    pub(super) rect: Rect,
    pub(super) items: &'a [CompletionLine],
    pub(super) kind_width: usize,
    pub(super) selected: Option<usize>,
}

pub(crate) struct PanelData {
    pub(super) rect: Rect,
    pub(super) lines: Vec<MarkdownLine>,
    pub(super) scroll: usize,
    selected: Option<usize>,
    title: Option<String>,
    status: Option<String>,
    focused: bool,
}

pub(crate) struct PluginPanelView<'a> {
    pub(crate) selected: Option<usize>,
    pub(crate) title: &'a str,
    pub(crate) status: &'a str,
    pub(crate) focused: bool,
}

impl PanelData {
    pub(super) fn new(rect: Rect, lines: Vec<MarkdownLine>, scroll: usize) -> Self {
        Self {
            rect,
            lines,
            scroll,
            selected: None,
            title: None,
            status: None,
            focused: false,
        }
    }

    pub(crate) fn plugin(
        rect: Rect,
        lines: Vec<MarkdownLine>,
        scroll: usize,
        view: PluginPanelView<'_>,
    ) -> Self {
        Self {
            rect,
            lines,
            scroll,
            selected: view.selected,
            title: Some(view.title.to_string()),
            status: Some(view.status.to_string()),
            focused: view.focused,
        }
    }
}

impl OverlayData<'_> {
    pub(crate) fn view(&self) -> Overlay<'_> {
        Overlay {
            list: self.list.as_ref().map(|list| OverlayList {
                rect: list.rect,
                items: list.items,
                kind_width: list.kind_width,
                selected: list.selected,
            }),
            panel: self.panel.as_ref().map(|panel| OverlayPanel {
                rect: panel.rect,
                lines: &panel.lines,
                scroll: panel.scroll,
                selected: panel.selected,
                title: panel.title.as_deref(),
                status: panel.status.as_deref(),
                focused: panel.focused,
            }),
            key_menu: self.key_menu.as_ref().map(keymenu::KeyMenuData::view),
        }
    }
}
