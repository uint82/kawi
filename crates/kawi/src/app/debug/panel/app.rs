use super::super::super::App;
use kawi_tui::frame::{DebugRow, Rect};

impl App {
    pub(crate) fn debug_panel_width(&self) -> usize {
        self.debug
            .panel_width(self.screen_width.saturating_sub(self.explorer_width()))
    }

    pub(crate) fn debug_panel_rect(&self) -> Option<Rect> {
        let width = self.debug_panel_width();
        if width == 0 {
            return None;
        }
        Some(Rect::new(
            self.screen_width.saturating_sub(width),
            0,
            width,
            self.screen_height.saturating_sub(1),
        ))
    }

    pub(crate) fn debug_panel_rows(&self) -> Vec<DebugRow> {
        self.debug
            .panel_entries()
            .into_iter()
            .map(|entry| DebugRow {
                depth: entry.depth,
                label: entry.label,
                kind: entry.kind,
            })
            .collect()
    }
}

impl App {
    pub(crate) fn handle_debug_panel_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> anyhow::Result<bool> {
        use crossterm::event::{KeyCode, KeyModifiers};
        let viewport = self
            .debug_panel_rect()
            .map(|rect| rect.height.saturating_sub(1))
            .unwrap_or(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.debug_focus = false,
            KeyCode::Char('x') => self.close_debug_panel(),
            KeyCode::Char('j') | KeyCode::Down => self.debug.move_panel_selection(1, viewport),
            KeyCode::Char('k') | KeyCode::Up => self.debug.move_panel_selection(-1, viewport),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.debug.move_panel_page(true, viewport)
            }
            KeyCode::PageDown => self.debug.move_panel_page(true, viewport),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.debug.move_panel_page(false, viewport)
            }
            KeyCode::PageUp => self.debug.move_panel_page(false, viewport),
            KeyCode::Char('g') | KeyCode::Home => self.debug.select_panel_start(viewport),
            KeyCode::Char('G') | KeyCode::End => self.debug.select_panel_end(viewport),
            KeyCode::Enter => {
                self.debug.activate_panel();
            }
            KeyCode::Char('r') => {
                self.debug.refresh_threads();
            }
            _ => {}
        }
        Ok(false)
    }

    pub(crate) fn toggle_debug_panel(&mut self) {
        if !self.debug.panel_visible() {
            self.debug.show_panel();
            self.debug_focus = true;
        } else if !self.debug_focus {
            self.debug_focus = true;
        } else {
            self.close_debug_panel();
            return;
        }
        if self.debug_panel_width() == 0 {
            self.debug.hide_panel();
            self.debug_focus = false;
            self.show_message("Debug panel needs more space".to_string());
        }
        self.sync_terminal_panel_size();
    }

    pub(crate) fn close_debug_panel(&mut self) {
        self.debug.hide_panel();
        self.debug_focus = false;
        self.sync_terminal_panel_size();
    }

    pub(crate) fn close_debug_panel_if_unfit(&mut self) {
        if self.debug.panel_visible() && self.debug_panel_width() == 0 {
            self.debug.hide_panel();
            self.debug_focus = false;
        }
    }
}
