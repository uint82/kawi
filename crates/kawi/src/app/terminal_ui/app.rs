use crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};
use kawi_tui::frame::Rect;

use super::{spawn_options, TerminalKey, TerminalLayout, TerminalUi};
use crate::app::{compact_error, App};
use crate::mouse::wheel_delta;

impl App {
    pub(crate) fn terminal_panel_rect(&self) -> Option<Rect> {
        match self.terminal.layout() {
            TerminalLayout::Horizontal => {
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
            TerminalLayout::Vertical => {
                let width = self.terminal.panel_width(self.screen_width);
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
        }
    }

    pub(crate) fn sync_terminal(&mut self) -> bool {
        let changed = self.terminal.drain();
        if let Some(status) = self.terminal.take_exit() {
            self.close_terminal();
            self.show_message(format!("Terminal: {status}"));
            return true;
        }
        changed
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

    pub(crate) fn open_terminal(
        &mut self,
        command: Option<String>,
        layout: TerminalLayout,
    ) -> anyhow::Result<bool> {
        if self.terminal.is_open() && command.is_none() {
            self.terminal.set_layout(layout);
            self.focus_terminal();
            self.sync_terminal_panel_size();
            self.clear_message();
            return Ok(false);
        }
        let Some((rows, cols)) = self.terminal_spawn_size(layout) else {
            self.show_message("Terminal pane needs more space".to_string());
            return Ok(false);
        };
        let options = spawn_options(self.task_root(), rows, cols, command);
        self.terminal.set_layout(layout);
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

    fn terminal_spawn_size(&self, layout: TerminalLayout) -> Option<(u16, u16)> {
        match layout {
            TerminalLayout::Horizontal => {
                let task_height = self.task.panel_height(self.screen_height);
                let height = TerminalUi::desired_panel_height(self.screen_height, task_height);
                if height == 0 {
                    return None;
                }
                Some((
                    height.saturating_sub(1).max(1) as u16,
                    self.screen_width.max(1) as u16,
                ))
            }
            TerminalLayout::Vertical => {
                let width = TerminalUi::desired_panel_width(self.screen_width);
                if width == 0 {
                    return None;
                }
                Some((
                    self.screen_height.saturating_sub(2).max(1) as u16,
                    width as u16,
                ))
            }
        }
    }

    pub(crate) fn toggle_terminal(&mut self) -> anyhow::Result<bool> {
        if !self.terminal.is_open() {
            return self.open_terminal(None, self.terminal.layout());
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
