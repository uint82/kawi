use kawi_tui::frame::Rect;

use super::super::terminal_ui::MIN_EDITOR_WIDTH;
use super::super::App;
use crate::options::{EXPLORER_WIDTH_MAX, EXPLORER_WIDTH_MIN};

impl App {
    pub(crate) fn explorer_width(&self) -> usize {
        if !self.explorer.is_enabled() || !self.explorer.is_visible() {
            return 0;
        }
        let available = self.screen_width.saturating_sub(MIN_EDITOR_WIDTH);
        if available < EXPLORER_WIDTH_MIN {
            return 0;
        }
        self.explorer
            .width()
            .clamp(EXPLORER_WIDTH_MIN, EXPLORER_WIDTH_MAX)
            .min(available)
    }

    pub(crate) fn close_explorer_if_unfit(&mut self) {
        if self.explorer.is_visible() && self.explorer_width() == 0 {
            self.explorer.close();
        }
    }

    pub(crate) fn explorer_rect(&self) -> Option<Rect> {
        let width = self.explorer_width();
        if width == 0 {
            return None;
        }
        let area = self.window_area();
        Some(Rect::new(0, 0, width, area.height))
    }

    pub(crate) fn terminal_panel_width(&self) -> usize {
        self.terminal
            .panel_width(self.screen_width.saturating_sub(self.explorer_width()))
    }

    pub(crate) fn explorer_content_height(&self) -> usize {
        let Some(rect) = self.explorer_rect() else {
            return 0;
        };
        let error_rows = usize::from(self.explorer.error().is_some());
        rect.height.saturating_sub(1).saturating_sub(error_rows)
    }
}
