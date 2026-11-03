use std::sync::Mutex;

use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, DecorationStyle, Error, Event};
use kawi_plugin_sdk::{ui_message, ui_panel, INVALID_PARAMS};

const OPEN: &str = "open";
const UPDATE: &str = "update";
const FOCUS: &str = "focus";
const CLOSE: &str = "close";

static PANEL: Mutex<Option<u64>> = Mutex::new(None);

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
