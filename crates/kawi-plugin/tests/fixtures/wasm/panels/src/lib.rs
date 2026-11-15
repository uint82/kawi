use std::sync::Mutex;

use kawi_plugin_sdk::exports::kawi::plugin::callback_handler::{Callback, Guest as CallbackGuest};
use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, DecorationStyle, Error, Event};
use kawi_plugin_sdk::{ui_message, ui_overlay, ui_panel, ui_picker, INVALID_PARAMS};

const OPEN: &str = "open";
const UPDATE: &str = "update";
const FOCUS: &str = "focus";
const CLOSE: &str = "close";
const OVERLAY_OPEN: &str = "overlay-open";
const OVERLAY_UPDATE: &str = "overlay-update";
const OVERLAY_WAIT: &str = "overlay-wait";
const OVERLAY_CLOSE: &str = "overlay-close";
const PICKER: &str = "picker";

static PANEL: Mutex<Option<u64>> = Mutex::new(None);
static OVERLAY: Mutex<Option<u64>> = Mutex::new(None);

struct Plugin;

impl Guest for Plugin {
    fn init(_host_protocol: u32) -> Result<(), Error> {
        Ok(())
    }

    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {
        match id.as_str() {
            OPEN => open(),
            UPDATE => update(),
            FOCUS => focus(),
            CLOSE => close(),
            OVERLAY_OPEN => overlay_open(),
            OVERLAY_UPDATE => overlay_update(),
            OVERLAY_WAIT => overlay_wait(),
            OVERLAY_CLOSE => overlay_close(),
            PICKER => picker(),
            _ => Err(error("unknown command")),
        }
    }

    fn on_event(_event: Event) {}
}

fn open() -> Result<Argument, Error> {
    let result = ui_panel::open(&ui_panel::OpenParams {
        title: "Index".to_string(),
        status: Some("building".to_string()),
        height: Some(6),
        focus: false,
        lines: vec![
            ui_panel::PanelLine {
                text: "alpha".to_string(),
                style: Some(DecorationStyle::Success),
            },
            ui_panel::PanelLine {
                text: "beta".to_string(),
                style: None,
            },
        ],
    })?;
    set_panel(Some(result.panel))?;
    message(&format!(
        "opened:{}:{}x{}",
        result.panel, result.width, result.height
    ))?;
    Ok(Argument::Null)
}

fn update() -> Result<Argument, Error> {
    ui_panel::update(&ui_panel::UpdateParams {
        panel: panel()?,
        title: None,
        status: Some("complete".to_string()),
        height: Some(5),
        lines: Some(vec![ui_panel::PanelLine {
            text: "gamma".to_string(),
            style: None,
        }]),
    })?;
    message("updated")?;
    Ok(Argument::Null)
}

fn focus() -> Result<Argument, Error> {
    ui_panel::focus(panel()?)?;
    message("focused")?;
    Ok(Argument::Null)
}

fn close() -> Result<Argument, Error> {
    ui_panel::close(panel()?)?;
    set_panel(None)?;
    message("closed")?;
    Ok(Argument::Null)
}

fn overlay_open() -> Result<Argument, Error> {
    let result = ui_overlay::open(&ui_overlay::OverlayOpenParams {
        title: "Search".to_string(),
        status: Some("ready".to_string()),
        anchor: ui_overlay::OverlayAnchor::Center,
        width: Some(30),
        height: Some(6),
        focus: true,
        selected: None,
        scroll: None,
        lines: vec![
            ui_overlay::OverlayLine {
                spans: vec![
                    ui_overlay::OverlaySpan {
                        text: "first".to_string(),
                        style: Some(DecorationStyle::Accent),
                    },
                    ui_overlay::OverlaySpan {
                        text: " line".to_string(),
                        style: None,
                    },
                ],
            },
            ui_overlay::OverlayLine {
                spans: vec![ui_overlay::OverlaySpan {
                    text: "second".to_string(),
                    style: None,
                }],
            },
        ],
    })?;
    set_overlay(Some(result.overlay))?;
    Ok(Argument::Null)
}

fn overlay_update() -> Result<Argument, Error> {
    ui_overlay::update(&ui_overlay::OverlayUpdateParams {
        overlay: overlay()?,
        title: Some("Search".to_string()),
        status: Some("updated".to_string()),
        anchor: None,
        width: None,
        height: None,
        focus: None,
        selected: Some(1),
        scroll: None,
        lines: Some(vec![ui_overlay::OverlayLine {
            spans: vec![ui_overlay::OverlaySpan {
                text: "third".to_string(),
                style: None,
            }],
        }]),
    })?;
    Ok(Argument::Null)
}

fn overlay_wait() -> Result<Argument, Error> {
    let text = match ui_overlay::wait(overlay()?)? {
        ui_overlay::OverlayEvent::Key(event) => match event.key {
            ui_overlay::Key::Character(value) => format!("key:{value}"),
            _ => "key:other".to_string(),
        },
        ui_overlay::OverlayEvent::Paste(text) => format!("paste:{text}"),
        ui_overlay::OverlayEvent::Closed(reason) => match reason {
            ui_overlay::OverlayCloseReason::Plugin => "closed:plugin".to_string(),
            ui_overlay::OverlayCloseReason::Escape => "closed:escape".to_string(),
            ui_overlay::OverlayCloseReason::Command => "closed:command".to_string(),
            ui_overlay::OverlayCloseReason::FocusLost => "closed:focus-lost".to_string(),
        },
    };
    message(&text)?;
    Ok(Argument::Null)
}

fn overlay_close() -> Result<Argument, Error> {
    ui_overlay::close(overlay()?)?;
    set_overlay(None)?;
    Ok(Argument::Null)
}

fn picker() -> Result<Argument, Error> {
    let outcome = ui_picker::pick(&ui_picker::PickerParams {
        title: "Pick".to_string(),
        items: vec![
            ui_picker::PickerItem {
                id: "a".to_string(),
                label: "Alpha".to_string(),
                detail: None,
                deprecated: false,
            },
            ui_picker::PickerItem {
                id: "b".to_string(),
                label: "Beta".to_string(),
                detail: Some("second".to_string()),
                deprecated: false,
            },
        ],
        selected: Some(1),
    })?;
    let text = match outcome {
        ui_picker::PickerOutcome::Selected(id) => format!("picked:{id}"),
        ui_picker::PickerOutcome::Canceled => "canceled".to_string(),
    };
    message(&text)?;
    Ok(Argument::Null)
}

fn overlay() -> Result<u64, Error> {
    OVERLAY
        .lock()
        .map_err(|_| error("overlay state poisoned"))?
        .ok_or_else(|| error("overlay is not open"))
}

fn set_overlay(value: Option<u64>) -> Result<(), Error> {
    *OVERLAY
        .lock()
        .map_err(|_| error("overlay state poisoned"))? = value;
    Ok(())
}

fn panel() -> Result<u64, Error> {
    PANEL
        .lock()
        .map_err(|_| error("panel state poisoned"))?
        .ok_or_else(|| error("panel is not open"))
}

fn set_panel(value: Option<u64>) -> Result<(), Error> {
    *PANEL.lock().map_err(|_| error("panel state poisoned"))? = value;
    Ok(())
}

fn message(text: &str) -> Result<(), Error> {
    ui_message::message(text)
}

fn error(message: &str) -> Error {
    Error {
        code: INVALID_PARAMS,
        message: message.to_string(),
        capability: None,
    }
}

kawi_plugin_sdk::export!(Plugin);

impl CallbackGuest for Plugin {
    fn on_callback(_callback: Callback) -> Result<(), Error> {
        Ok(())
    }
}
