use std::sync::Mutex;

use kawi_plugin_sdk::exports::kawi::plugin::callback_handler::{
    Callback, CallbackEvent, Guest as CallbackGuest,
};
use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, Error, Event};
use kawi_plugin_sdk::{editor_autocmds, ui_message};

const PROTOCOL_VERSION: u32 = 1;
const INVALID_PARAMS: i32 = -32602;

static REGISTRATION: Mutex<Option<u64>> = Mutex::new(None);
static CALLBACKS: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Plugin;

impl Guest for Plugin {
    fn init(host_protocol: u32) -> Result<(), Error> {
        if host_protocol == PROTOCOL_VERSION {
            Ok(())
        } else {
            Err(error("unsupported host protocol version"))
        }
    }

    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {
        match id.as_str() {
            "register" => register(false),
            "register-once" => register(true),
            "register-user" => register_user(),
            "remove" => remove(),
            "count" => count(),
            "last" => last(),
            "emit" => emit(),
            "clear" => clear(),
            _ => Err(error("unknown command")),
        }
    }

    fn on_event(_event: Event) {}
}

impl CallbackGuest for Plugin {
    fn on_callback(callback: Callback) -> Result<(), Error> {
        let Callback::Autocmd(callback) = callback else {
            return Ok(());
        };
        let text = match callback.event {
            CallbackEvent::Legacy(Event::FileSaved(event)) => format!(
                "{}:{}:{}",
                callback.registration,
                event.buffer,
                event.path.unwrap_or_default()
            ),
            CallbackEvent::LanguageChanged(event) => format!(
                "{}:language:{}",
                callback.registration,
                event.language.unwrap_or_default()
            ),
            CallbackEvent::User(event) => format!(
                "{}:user:{}:{}",
                callback.registration,
                event.name,
                event.payload.unwrap_or_default()
            ),
            _ => format!("{}:other", callback.registration),
        };
        CALLBACKS.lock().unwrap().push(text);
        Ok(())
    }
}

fn register(once: bool) -> Result<Argument, Error> {
    let registered = editor_autocmds::register(&editor_autocmds::AutocmdSpec {
        event: "file_saved".to_string(),
        pattern: Some("*.rs".to_string()),
        buffer: None,
        once,
    })?;
    *REGISTRATION.lock().unwrap() = Some(registered.id);
    ui_message::message(&registered.id.to_string())?;
    Ok(Argument::Text(registered.id.to_string()))
}

fn register_user() -> Result<Argument, Error> {
    let registered = editor_autocmds::register(&editor_autocmds::AutocmdSpec {
        event: "user".to_string(),
        pattern: Some("custom".to_string()),
        buffer: None,
        once: false,
    })?;
    *REGISTRATION.lock().unwrap() = Some(registered.id);
    ui_message::message(&registered.id.to_string())?;
    Ok(Argument::Text(registered.id.to_string()))
}

fn remove() -> Result<Argument, Error> {
    let id = REGISTRATION
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| error("autocmd is not registered"))?;
    editor_autocmds::remove(id)?;
    ui_message::message("removed")?;
    Ok(Argument::Null)
}

fn count() -> Result<Argument, Error> {
    let count = CALLBACKS.lock().unwrap().len();
    ui_message::message(&count.to_string())?;
    Ok(Argument::Text(count.to_string()))
}

fn last() -> Result<Argument, Error> {
    let last = CALLBACKS
        .lock()
        .unwrap()
        .last()
        .cloned()
        .unwrap_or_default();
    ui_message::message(&last)?;
    Ok(Argument::Text(last))
}

fn emit() -> Result<Argument, Error> {
    editor_autocmds::emit_user_event("custom", Some("data"))?;
    Ok(Argument::Null)
}

fn clear() -> Result<Argument, Error> {
    editor_autocmds::clear()?;
    *REGISTRATION.lock().unwrap() = None;
    ui_message::message("cleared")?;
    Ok(Argument::Null)
}

fn error(message: &str) -> Error {
    Error {
        code: INVALID_PARAMS,
        message: message.to_string(),
        capability: None,
    }
}

kawi_plugin_sdk::export!(Plugin);
