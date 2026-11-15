use super::bindings::kawi::plugin::types::Error;
use super::bindings::kawi::plugin::ui_overlay::{
    self, Key, KeyEvent, KeyModifier, OverlayCloseReason, OverlayEvent, OverlayLine, OverlaySpan,
};
use super::convert::{decoration_style_to_protocol, error_from_rpc};
use super::state::{decode_json, serialize_params, WasmState};
use crate::protocol::{
    Key as ProtocolKey, KeyModifier as ProtocolKeyModifier, OverlayAnchor,
    OverlayCloseReason as ProtocolOverlayCloseReason, OverlayEvent as ProtocolOverlayEvent,
    OverlayKeyEvent, OverlayLine as ProtocolOverlayLine, OverlayOpenParams, OverlayOpenResult,
    OverlayParams, OverlaySpan as ProtocolOverlaySpan, OverlayUpdateParams,
};

impl ui_overlay::Host for WasmState {
    fn open(
        &mut self,
        params: ui_overlay::OverlayOpenParams,
    ) -> Result<ui_overlay::OverlayOpenResult, Error> {
        let params = open_params(params);
        let value = self
            .bridge
            .request("ui.overlay_open", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let result: OverlayOpenResult = decode_json(value)?;
        Ok(ui_overlay::OverlayOpenResult {
            overlay: result.overlay,
            width: result.width,
            height: result.height,
        })
    }

    fn update(&mut self, params: ui_overlay::OverlayUpdateParams) -> Result<(), Error> {
        let params = update_params(params);
        self.bridge
            .request("ui.overlay_update", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn wait(&mut self, overlay: u64) -> Result<OverlayEvent, Error> {
        let params = OverlayParams { overlay };
        let value = self
            .bridge
            .request_interactive("ui.overlay_wait", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let event: ProtocolOverlayEvent = decode_json(value)?;
        Ok(event_to_wit(event))
    }

    fn close(&mut self, overlay: u64) -> Result<(), Error> {
        let params = OverlayParams { overlay };
        self.bridge
            .request("ui.overlay_close", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }
}

fn open_params(params: ui_overlay::OverlayOpenParams) -> OverlayOpenParams {
    OverlayOpenParams {
        title: params.title,
        status: params.status,
        anchor: anchor_to_protocol(params.anchor),
        width: params.width,
        height: params.height,
        focus: params.focus,
        selected: params.selected,
        scroll: params.scroll,
        lines: lines_to_protocol(params.lines),
    }
}

fn update_params(params: ui_overlay::OverlayUpdateParams) -> OverlayUpdateParams {
    OverlayUpdateParams {
        overlay: params.overlay,
        title: params.title,
        status: params.status,
        anchor: params.anchor.map(anchor_to_protocol),
        width: params.width,
        height: params.height,
        focus: params.focus,
        selected: params.selected,
        scroll: params.scroll,
        lines: params.lines.map(lines_to_protocol),
    }
}

fn lines_to_protocol(lines: Vec<OverlayLine>) -> Vec<ProtocolOverlayLine> {
    lines
        .into_iter()
        .map(|line| ProtocolOverlayLine {
            spans: line.spans.into_iter().map(span_to_protocol).collect(),
        })
        .collect()
}

fn span_to_protocol(span: OverlaySpan) -> ProtocolOverlaySpan {
    ProtocolOverlaySpan {
        text: span.text,
        style: span.style.map(decoration_style_to_protocol),
    }
}

fn anchor_to_protocol(anchor: ui_overlay::OverlayAnchor) -> OverlayAnchor {
    match anchor {
        ui_overlay::OverlayAnchor::Cursor => OverlayAnchor::Cursor,
        ui_overlay::OverlayAnchor::Center => OverlayAnchor::Center,
        ui_overlay::OverlayAnchor::TopLeft => OverlayAnchor::TopLeft,
        ui_overlay::OverlayAnchor::TopRight => OverlayAnchor::TopRight,
        ui_overlay::OverlayAnchor::BottomLeft => OverlayAnchor::BottomLeft,
        ui_overlay::OverlayAnchor::BottomRight => OverlayAnchor::BottomRight,
    }
}

fn event_to_wit(event: ProtocolOverlayEvent) -> OverlayEvent {
    match event {
        ProtocolOverlayEvent::Key(key) => OverlayEvent::Key(key_to_wit(key)),
        ProtocolOverlayEvent::Paste(text) => OverlayEvent::Paste(text),
        ProtocolOverlayEvent::Closed(reason) => OverlayEvent::Closed(close_reason_to_wit(reason)),
    }
}

fn key_to_wit(event: OverlayKeyEvent) -> KeyEvent {
    KeyEvent {
        key: match event.key {
            ProtocolKey::Character(value) => Key::Character(value),
            ProtocolKey::Enter => Key::Enter,
            ProtocolKey::Escape => Key::Escape,
            ProtocolKey::Tab => Key::Tab,
            ProtocolKey::Backspace => Key::Backspace,
            ProtocolKey::Delete => Key::Delete,
            ProtocolKey::Insert => Key::Insert,
            ProtocolKey::Left => Key::Left,
            ProtocolKey::Right => Key::Right,
            ProtocolKey::Up => Key::Up,
            ProtocolKey::Down => Key::Down,
            ProtocolKey::Home => Key::Home,
            ProtocolKey::End => Key::End,
            ProtocolKey::PageUp => Key::PageUp,
            ProtocolKey::PageDown => Key::PageDown,
            ProtocolKey::Function(value) => Key::Function(value),
            ProtocolKey::Other(value) => Key::Other(value),
        },
        modifiers: event.modifiers.into_iter().map(modifier_to_wit).collect(),
        text: event.text,
    }
}

fn modifier_to_wit(modifier: ProtocolKeyModifier) -> KeyModifier {
    match modifier {
        ProtocolKeyModifier::Shift => KeyModifier::Shift,
        ProtocolKeyModifier::Control => KeyModifier::Control,
        ProtocolKeyModifier::Alt => KeyModifier::Alt,
        ProtocolKeyModifier::Super => KeyModifier::Super,
        ProtocolKeyModifier::Hyper => KeyModifier::Hyper,
        ProtocolKeyModifier::Meta => KeyModifier::Meta,
    }
}

fn close_reason_to_wit(reason: ProtocolOverlayCloseReason) -> OverlayCloseReason {
    match reason {
        ProtocolOverlayCloseReason::Plugin => OverlayCloseReason::Plugin,
        ProtocolOverlayCloseReason::Escape => OverlayCloseReason::Escape,
        ProtocolOverlayCloseReason::Command => OverlayCloseReason::Command,
        ProtocolOverlayCloseReason::FocusLost => OverlayCloseReason::FocusLost,
    }
}
