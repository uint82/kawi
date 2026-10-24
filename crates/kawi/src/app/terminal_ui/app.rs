use crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};
use kawi_tui::frame::Rect;

use super::{spawn_options, TerminalKey, TerminalUi};
use crate::app::{compact_error, App};
use crate::mouse::wheel_delta;

impl App {
    pub(crate) fn terminal_panel_rect(&self) -> Option<Rect> {
        let height = self.terminal.panel_height(
            self.screen_height,
            self.task.panel_height(self.screen_height),
        );
        if height == 0 {
            return None;
        }
        Some(Rect::new(
            0,
            self.screen_height.saturating_sub(1).saturating_sub(height),
            self.screen_width,
            height,
        ))
    }

    pub(crate) fn sync_terminal(&mut self) -> bool {
        self.terminal.drain()
    }

    pub(crate) fn sync_terminal_panel_size(&mut self) {
        let Some(rect) = self.terminal_panel_rect() else {
            if self.terminal.is_open() {
                self.terminal_focus = false;
                self.terminal.close();
            }
            self.refresh_window_sizes();
            self.reveal_cursor();
            return;
        };
        let rows = rect.height.saturating_sub(1).max(1) as u16;
        let cols = rect.width.max(1) as u16;
        if let Err(error) = self.terminal.resize(rows, cols) {
            self.show_message(format!(
                "Terminal resize failed: {}",
                compact_error(&error.to_string())
            ));
        }
        self.refresh_window_sizes();
        self.reveal_cursor();
    }

    pub(crate) fn open_terminal(&mut self, command: Option<String>) -> anyhow::Result<bool> {
        let task_height = self.task.panel_height(self.screen_height);
        let height = TerminalUi::desired_panel_height(self.screen_height, task_height);
        if height == 0 {
            self.show_message("Terminal pane needs more space".to_string());
            return Ok(false);
        }
        let rows = height.saturating_sub(1).max(1) as u16;
        let cols = self.screen_width.max(1) as u16;
        let options = spawn_options(self.task_root(), rows, cols, command);
        match self.terminal.open(options, self.events.wake()) {
            Ok(()) => {
                self.terminal_focus = true;
                self.sync_terminal_panel_size();
                self.clear_message();
            }
            Err(error) => {
                self.terminal_focus = false;
                self.show_message(format!(
                    "Terminal failed to start: {}",
                    compact_error(&error.to_string())
                ));
            }
        }
        Ok(false)
    }

    pub(crate) fn toggle_terminal(&mut self) -> anyhow::Result<bool> {
        if !self.terminal.is_open() {
            return self.open_terminal(None);
        }
        if self.terminal_focus {
            self.close_terminal();
            return Ok(false);
        }
        self.focus_terminal();
        Ok(false)
    }

    pub(crate) fn focus_terminal(&mut self) {
        if !self.terminal.is_open() {
            return;
        }
        self.terminal_focus = true;
        if self.terminal.is_running() {
            self.terminal.focus_input();
        }
    }

    pub(crate) fn close_terminal(&mut self) {
        self.terminal_focus = false;
        self.terminal.close();
        self.sync_terminal_panel_size();
    }

    pub(crate) fn handle_terminal_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if !self.terminal.is_open() {
            self.terminal_focus = false;
            return Ok(false);
        }
        if !self.terminal_focus {
            return Ok(false);
        }
        let Some(rect) = self.terminal_panel_rect() else {
            self.terminal_focus = false;
            return Ok(false);
        };
        let rows = rect.height.saturating_sub(1);
        match self.terminal.handle_key(key, rows) {
            TerminalKey::Consumed => Ok(false),
            TerminalKey::ClosePanel => {
                self.close_terminal();
                Ok(false)
            }
            TerminalKey::FocusEditor => {
                self.terminal_focus = false;
                self.reveal_cursor();
                Ok(false)
            }
        }
    }

    pub(crate) fn handle_terminal_paste(&mut self, text: &str) -> bool {
        if !self.terminal_focus || !self.terminal.is_open() {
            return false;
        }
        self.terminal.handle_paste(text);
        true
    }

    pub(crate) fn handle_terminal_mouse(&mut self, mouse: MouseEvent) -> Option<bool> {
        let rect = self.terminal_panel_rect()?;
        let column = usize::from(mouse.column);
        let row = usize::from(mouse.row);
        if !rect.contains(column, row) {
            return None;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                self.terminal_focus = true;
                if let Some(delta) = wheel_delta(&mouse) {
                    self.terminal.scroll_wheel(delta);
                }
                Some(false)
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                self.focus_terminal();
                Some(false)
            }
            _ => Some(false),
        }
    }
}
