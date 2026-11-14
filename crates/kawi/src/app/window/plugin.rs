use super::WindowId;
use crate::app::App;
use kawi_tui::frame::Rect;

impl App {
    pub(crate) fn open_buffer_window_direction(&mut self, number: usize, vertical: bool) -> bool {
        self.begin_split();
        if !self.editor.buffer_select(number) {
            return false;
        }
        self.complete_split(vertical).is_ok()
    }

    pub(crate) fn close_window(&mut self, id: WindowId, force: bool) -> Result<bool, String> {
        if self.windows.view(id).is_none() {
            return Err(format!("window not found: {id}"));
        }
        if self.windows.len() <= 1 {
            return Err("cannot close the last window".to_string());
        }
        if self.windows.active() != id && !self.focus_window(id) {
            return Err(format!("cannot focus window: {id}"));
        }
        self.close_active_window(force)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn window_info_at(&self, id: WindowId) -> Option<(usize, Rect, bool)> {
        let view = self.windows.view(id)?;
        let rect = self
            .window_layout()
            .panes
            .iter()
            .find(|(window, _)| *window == id)
            .map(|(_, rect)| *rect)?;
        Some((view.buffer, rect, self.windows.active() == id))
    }

    pub(crate) fn window_ids(&self) -> Vec<WindowId> {
        self.windows.ids()
    }
}
