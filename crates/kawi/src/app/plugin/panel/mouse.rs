use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;
use crate::mouse::wheel_delta;

impl App {
    pub(crate) fn handle_plugin_panel_mouse(&mut self, mouse: MouseEvent) -> Option<bool> {
        if self.pager.is_some() {
            return None;
        }
        let rect = self.plugin_panel_rect()?;
        let column = usize::from(mouse.column);
        let row = usize::from(mouse.row);
        if !rect.contains(column, row) {
            return None;
        }
        if let Some(delta) = wheel_delta(&mouse) {
            let viewport = rect.height.saturating_sub(1).max(1);
            self.plugin_panel.scroll(delta, viewport);
            return Some(false);
        }
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            self.focus_plugin_panel();
            return Some(false);
        }
        None
    }
}
