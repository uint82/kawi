use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, Error, Event, SettingValue};
use kawi_plugin_sdk::{config_read, editor_execute, ui_message};

const PROTOCOL_VERSION: u32 = 1;
const INVALID_PARAMS: i32 = -32602;

struct Config;

impl Guest for Config {
    fn init(host_protocol: u32) -> Result<(), Error> {
        if host_protocol == PROTOCOL_VERSION {
            Ok(())
        } else {
            Err(error(INVALID_PARAMS, "unsupported host protocol version"))
        }
    }

    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {
        match id.as_str() {
            "read" => read_setting(),
            "list" => list_settings(),
            "execute" => execute_command(),
            "commands" => list_commands(),
            _ => Err(error(INVALID_PARAMS, "unknown command")),
        }
    }

    fn on_event(_event: Event) {}
}

fn read_setting() -> Result<Argument, Error> {
    let value = config_read::get("size")?.unwrap_or(SettingValue::Text("missing".to_string()));
    message(&value_text(value))
}

fn list_settings() -> Result<Argument, Error> {
    let text = config_read::get_all()?
        .into_iter()
        .map(|setting| format!("{}={}", setting.key, value_text(setting.value)))
        .collect::<Vec<_>>()
        .join(" ");
    message(&text)
}

fn execute_command() -> Result<Argument, Error> {
    editor_execute::execute("mode.insert")?;
    message("executed")
}

fn list_commands() -> Result<Argument, Error> {
    let text = editor_execute::list_commands()?
        .into_iter()
        .map(|command| command.id)
        .collect::<Vec<_>>()
        .join(" ");
    message(&text)
}

fn message(text: &str) -> Result<Argument, Error> {
    ui_message::message(text)?;
    Ok(Argument::Null)
}

fn value_text(value: SettingValue) -> String {
    match value {
        SettingValue::Boolean(value) => value.to_string(),
        SettingValue::Integer(value) => value.to_string(),
        SettingValue::Float(value) => value.to_string(),
        SettingValue::Text(value) => value,
    }
}

fn error(code: i32, message: &str) -> Error {
    Error {
        code,
        message: message.to_string(),
        capability: None,
    }
}

kawi_plugin_sdk::export!(Config);
