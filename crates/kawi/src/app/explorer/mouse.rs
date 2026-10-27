use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::super::App;
use crate::mouse::wheel_delta;

impl App {
    pub(crate) fn explorer_contains_mouse(&self, mouse: &MouseEvent) -> bool {
        let Some(rect) = self.explorer_rect() else {
            return false;
        };
        rect.contains(usize::from(mouse.column), usize::from(mouse.row))
    }

    pub(crate) fn handle_explorer_mouse(&mut self, mouse: MouseEvent) -> anyhow::Result<bool> {
        let Some(rect) = self.explorer_rect() else {
            return Ok(false);
        };
        let height = self.explorer_content_height().max(1);
        let row = usize::from(mouse.row);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.explorer.focus();
                if row <= rect.y {
                    return Ok(false);
                }
                let relative = row.saturating_sub(rect.y).saturating_sub(1);
                if relative >= height {
                    return Ok(false);
                }
                let index = relative.saturating_add(self.explorer.offset());
                let Some(entry) = self.explorer.rows().get(index) else {
                    return Ok(false);
                };
                let path = entry.path.clone();
                let double = self.explorer.register_click(&path);
                self.explorer.select_at(index, height);
                if double {
                    return self.open_explorer_selected(None);
                }
                Ok(false)
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                self.explorer.focus();
                if let Some(delta) = wheel_delta(&mouse) {
                    self.explorer.move_selection(delta, height);
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}
