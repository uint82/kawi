use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_plugin::{Key, KeyModifier, OverlayCloseReason, OverlayEvent, OverlayKeyEvent};

use crate::app::App;

impl App {
    pub(crate) fn handle_plugin_overlay_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if key.code == KeyCode::Esc
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.close_plugin_overlay(OverlayCloseReason::Escape);
            return Ok(false);
        }
        let event = OverlayEvent::Key(overlay_key_event(key));
        self.emit_plugin_overlay_event(event);
        Ok(false)
    }

    pub(crate) fn close_plugin_overlay(&mut self, reason: OverlayCloseReason) -> bool {
        if !self.plugin_overlays.has_overlay() {
            return false;
        }
        if let Some((waiter, event)) = self.plugin_overlays.close_active(reason) {
            self.respond_plugin_overlay(waiter, event);
        }
        true
    }

    pub(crate) fn emit_plugin_overlay_event(&mut self, event: OverlayEvent) {
        let Some((waiter, event)) = self.plugin_overlays.emit(event) else {
            return;
        };
        self.respond_plugin_overlay(waiter, event);
    }

    pub(crate) fn respond_plugin_overlay(
        &mut self,
        waiter: super::state::OverlayWaiter,
        event: OverlayEvent,
    ) {
        let value = serde_json::to_value(event).unwrap_or(serde_json::Value::Null);
        let _ = self
            .plugins
            .respond(&waiter.plugin, waiter.request, Ok(value));
    }

    pub(crate) fn paste_plugin_overlay(&mut self, text: &str) -> bool {
        if !self.plugin_overlays.is_focused() {
            return false;
        }
        self.emit_plugin_overlay_event(OverlayEvent::Paste(text.to_string()));
        true
    }
}

fn overlay_key_event(key: KeyEvent) -> OverlayKeyEvent {
    let text = match key.code {
        KeyCode::Char(value) => Some(value.to_string()),
        _ => None,
    };
    OverlayKeyEvent {
        key: key_code(key.code),
        modifiers: key_modifiers(key.modifiers),
        text,
    }
}

fn key_code(code: KeyCode) -> Key {
    match code {
        KeyCode::Char(value) => Key::Character(value),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::F(value) => Key::Function(value),
        other => Key::Other(format!("{other:?}")),
    }
}

fn key_modifiers(modifiers: KeyModifiers) -> Vec<KeyModifier> {
    let mut result = Vec::new();
    if modifiers.contains(KeyModifiers::SHIFT) {
        result.push(KeyModifier::Shift);
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        result.push(KeyModifier::Control);
    }
    if modifiers.contains(KeyModifiers::ALT) {
        result.push(KeyModifier::Alt);
    }
    if modifiers.contains(KeyModifiers::SUPER) {
        result.push(KeyModifier::Super);
    }
    if modifiers.contains(KeyModifiers::HYPER) {
        result.push(KeyModifier::Hyper);
    }
    if modifiers.contains(KeyModifiers::META) {
        result.push(KeyModifier::Meta);
    }
    result
}
