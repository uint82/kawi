use super::super::color_picker::render;
use super::super::App;
use super::size::size_panel;
use super::{OverlayData, PanelData};

const MAX_WIDTH: usize = 56;
const MIN_WIDTH: usize = 34;

impl App {
    pub(super) fn color_picker_overlay(&self) -> Option<OverlayData<'_>> {
        let picker = self.color_picker.as_ref()?;
        let limits = self.cursor_limits(MAX_WIDTH, MIN_WIDTH);
        let panel = size_panel(limits, |width| render::lines(picker, &self.theme, width));
        let layout = self.panel_layout(panel);
        Some(OverlayData {
            list: None,
            panel: Some(PanelData {
                rect: layout.rect,
                lines: layout.lines,
                scroll: 0,
            }),
            key_menu: None,
        })
    }
}
