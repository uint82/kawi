use kawi_plugin_sdk::exports::kawi::plugin::callback_handler::{Callback, Guest as CallbackGuest};
use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, Error, Event, TimerFired, VariableScope, VariableTarget};
use kawi_plugin_sdk::{editor_schedule, editor_variables, options_write, ui_message};

const PROTOCOL_VERSION: u32 = 1;
const INVALID_PARAMS: i32 = -32602;
const EVERY_INTERVAL_MS: u32 = 60_000;
const AFTER_DELAY_MS: u32 = 60_000;

struct Plugin;

impl Guest for Plugin {
    fn init(host_protocol: u32) -> Result<(), Error> {
        if host_protocol != PROTOCOL_VERSION {
            return Err(error("unsupported host protocol version"));
        }
        editor_variables::set(global_target(), "ready", &Argument::Text("yes".to_string()))?;
        match editor_variables::get(global_target(), "ready")? {
            Some(Argument::Text(value)) if value == "yes" => {}
            _ => return Err(error("global variable did not round trip")),
        }
        editor_variables::set(plugin_target(), "count", &Argument::Integer(1))?;
        let variables = editor_variables::list_variables(plugin_target())?;
        if variables.len() != 1 || variables[0].key != "count" {
            return Err(error("plugin variables did not round trip"));
        }
        let option = options_write::set("number", &Argument::Boolean(false))?;
        if option.value != "false" {
            return Err(error("option did not change"));
        }
        let option = options_write::reset("number")?;
        if option.value != "true" {
            return Err(error("option did not reset"));
        }
        editor_schedule::after(AFTER_DELAY_MS, Some("later"))?;
        editor_schedule::every(EVERY_INTERVAL_MS, Some("tick"))?;
        if editor_schedule::list_timers()?.len() != 2 {
            return Err(error("timers were not registered"));
        }
        Ok(())
    }

    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {
        match id.as_str() {
            "report" => report(),
            "fire" => fire(),
            "cancel" => cancel(),
            _ => Err(error("unknown command")),
        }
    }

    fn on_event(_event: Event) {}
}

impl CallbackGuest for Plugin {
    fn on_callback(callback: Callback) -> Result<(), Error> {
        let Callback::Timer(TimerFired { timer: _, payload }) = callback else {
            return Ok(());
        };
        editor_variables::set(
            plugin_target(),
            "last",
            &Argument::Text(payload.unwrap_or_default()),
        )?;
        Ok(())
    }
}

fn global_target() -> VariableTarget {
    VariableTarget {
        scope: VariableScope::Global,
        buffer: None,
        window: None,
    }
}

fn plugin_target() -> VariableTarget {
    VariableTarget {
        scope: VariableScope::Plugin,
        buffer: None,
        window: None,
    }
}

fn report() -> Result<Argument, Error> {
    let ready = match editor_variables::get(global_target(), "ready")? {
        Some(Argument::Text(text)) => text,
        _ => "missing".to_string(),
    };
    let variables = editor_variables::list_variables(plugin_target())?;
    let timers = editor_schedule::list_timers()?;
    let last = match editor_variables::get(plugin_target(), "last")? {
        Some(Argument::Text(text)) => text,
        _ => "none".to_string(),
    };
    let text = format!(
        "ready={ready};variables={};timers={};last={last}",
        variables.len(),
        timers.len()
    );
    ui_message::message(&text)?;
    Ok(Argument::Text(text))
}

fn fire() -> Result<Argument, Error> {
    editor_schedule::after(0, Some("once"))?;
    ui_message::message("scheduled")?;
    Ok(Argument::Null)
}

fn cancel() -> Result<Argument, Error> {
    let mut cancelled = 0;
    for timer in editor_schedule::list_timers()? {
        if editor_schedule::cancel(timer.id)? {
            cancelled += 1;
        }
    }
    ui_message::message(&cancelled.to_string())?;
    Ok(Argument::Integer(cancelled))
}

fn error(message: &str) -> Error {
    Error {
        code: INVALID_PARAMS,
        message: message.to_string(),
        capability: None,
    }
}

kawi_plugin_sdk::export!(Plugin);
