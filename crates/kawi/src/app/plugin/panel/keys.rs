use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;

impl App {
    pub(crate) fn handle_plugin_panel_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let Some(rect) = self.plugin_panel_rect() else {
            self.plugin_panel.unfocus();
            return Ok(false);
        };
        let viewport = rect.height.saturating_sub(1).max(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.unfocus_plugin_panel();
            }
            KeyCode::Char('x') => {
                self.close_plugin_panel();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.plugin_panel.scroll(1, viewport);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.plugin_panel.scroll(-1, viewport);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.plugin_panel.scroll_page(1, viewport);
            }
            KeyCode::PageDown => {
                self.plugin_panel.scroll_page(1, viewport);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.plugin_panel.scroll_page(-1, viewport);
            }
            KeyCode::PageUp => {
                self.plugin_panel.scroll_page(-1, viewport);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.plugin_panel.scroll_start();
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.plugin_panel.scroll_end(viewport);
            }
            _ => {}
        }
        Ok(false)
    }

    pub(crate) fn toggle_plugin_panel_focus(&mut self) -> anyhow::Result<bool> {
        if self.plugin_panel.active().is_none() {
            self.show_message("No plugin panel".to_string());
            return Ok(false);
        }
        if self.plugin_panel.is_focused() {
            self.unfocus_plugin_panel();
        } else if !self.focus_plugin_panel() {
            self.show_message("Plugin panel needs more space".to_string());
        }
        Ok(false)
    }

    pub(crate) fn handle_plugin_panel_command(&mut self, rest: &str) {
        match rest.trim() {
            "" | "focus" => {
                let _ = self.toggle_plugin_panel_focus();
            }
            "close" => {
                if !self.close_plugin_panel() {
                    self.show_message("No plugin panel".to_string());
                }
            }
            _ => self.show_message("Usage: plugin panel [close]".to_string()),
        }
    }
}
