use kawi_tui::frame::{PluginPanel, Rect};

use super::super::super::App;
use kawi_plugin::MIN_PLUGIN_PANEL_HEIGHT;

const MIN_EDITOR_ROWS: usize = 4;

impl App {
    pub(crate) fn plugin_panel_height(&self) -> usize {
        let Some(requested) = self.plugin_panel.requested_height() else {
            return 0;
        };
        let task_height = self.task.panel_height(self.screen_height);
        let terminal_height = self.terminal.panel_height(self.screen_height, task_height);
        let available = self
            .screen_height
            .saturating_sub(1)
            .saturating_sub(task_height)
            .saturating_sub(terminal_height)
            .saturating_sub(MIN_EDITOR_ROWS);
        if available < MIN_PLUGIN_PANEL_HEIGHT as usize {
            return 0;
        }
        requested.min(available)
    }

    pub(crate) fn plugin_panel_rect(&self) -> Option<Rect> {
        let height = self.plugin_panel_height();
        if height == 0 {
            return None;
        }
        let task_height = self.task.panel_height(self.screen_height);
        let terminal_height = self.terminal.panel_height(self.screen_height, task_height);
        let explorer_width = self.explorer_width();
        let terminal_width = self.terminal_panel_width();
        let debug_width = self.debug_panel_width();
        let width = self
            .screen_width
            .saturating_sub(explorer_width)
            .saturating_sub(terminal_width)
            .saturating_sub(debug_width);
        if width == 0 {
            return None;
        }
        Some(Rect::new(
            explorer_width,
            self.screen_height
                .saturating_sub(1)
                .saturating_sub(terminal_height)
                .saturating_sub(task_height)
                .saturating_sub(height),
            width,
            height,
        ))
    }

    pub(crate) fn plugin_panel_view(&self) -> Option<PluginPanel<'_>> {
        let rect = self.plugin_panel_rect()?;
        let panel = self.plugin_panel.active()?;
        let viewport = rect.height.saturating_sub(1).max(1);
        Some(PluginPanel {
            rect,
            title: panel.title,
            status: panel.status,
            focused: panel.focused,
            lines: panel.lines,
            offset: self.plugin_panel.clamped_offset(viewport),
        })
    }

    pub(crate) fn focus_plugin_panel(&mut self) -> bool {
        if self.plugin_panel_rect().is_none() || self.plugin_panel.active().is_none() {
            return false;
        }
        self.cancel_pending_input();
        self.clear_focus();
        self.plugin_panel.focus_active()
    }

    pub(crate) fn unfocus_plugin_panel(&mut self) -> bool {
        if !self.plugin_panel.unfocus() {
            return false;
        }
        self.reveal_cursor();
        true
    }

    pub(crate) fn close_plugin_panel(&mut self) -> bool {
        if !self.plugin_panel.close_active() {
            return false;
        }
        self.sync_plugin_panel_size();
        true
    }

    pub(crate) fn sync_plugin_panel_size(&mut self) {
        if self.plugin_panel_height() == 0 {
            self.plugin_panel.unfocus();
        }
        self.sync_terminal_panel_size();
        if let Some(rect) = self.plugin_panel_rect() {
            self.plugin_panel
                .clamp_offset(rect.height.saturating_sub(1).max(1));
        }
    }

    pub(crate) fn close_plugin_panel_if_unfit(&mut self) {
        if self.plugin_panel_height() == 0 {
            self.plugin_panel.unfocus();
        }
    }
}
