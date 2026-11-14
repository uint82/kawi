use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, Error, Event};
use kawi_plugin_sdk::{editor_commands, editor_keymaps, ui_message};

const PROTOCOL_VERSION: u32 = 1;
const INVALID_PARAMS: i32 = -32602;
const COMMAND_ID: &str = "greet";

struct Plugin;

impl Guest for Plugin {
    fn init(host_protocol: u32) -> Result<(), Error> {
        if host_protocol != PROTOCOL_VERSION {
            return Err(error("unsupported host protocol version"));
        }
        let registered = editor_commands::register(&editor_commands::CommandSpec {
            id: COMMAND_ID.to_string(),
            title: "Greet".to_string(),
            description: Some("Greet the editor".to_string()),
            aliases: vec!["hello".to_string()],
        })?;
        if !registered.name.ends_with(".greet") {
            return Err(error("unexpected command name"));
        }
        if !editor_commands::list_registered()?
            .iter()
            .any(|command| command.id == COMMAND_ID)
        {
            return Err(error("registered command is missing"));
        }
        editor_keymaps::set(&editor_keymaps::KeymapSpec {
            mode: editor_keymaps::KeyMode::Normal,
            keys: "<leader>pg".to_string(),
            command: registered.name,
            description: Some("Greet".to_string()),
        })?;
        if !editor_keymaps::list_registered()?
            .iter()
            .any(|keymap| keymap.keys == "<leader>pg")
        {
            return Err(error("registered keymap is missing"));
        }
        Ok(())
    }

    fn invoke_command(id: String, args: Vec<Argument>) -> Result<Argument, Error> {
        match id.as_str() {
            COMMAND_ID => greet(args),
            "ping" => ping(),
            "unregister" => unregister(),
            _ => Err(error("unknown command")),
        }
    }

    fn on_event(_event: Event) {}
}

fn greet(args: Vec<Argument>) -> Result<Argument, Error> {
    let text = args
        .into_iter()
        .find_map(|argument| match argument {
            Argument::Text(text) => Some(text),
            _ => None,
        })
        .unwrap_or_else(|| "none".to_string());
    ui_message::message(&format!("greet:{text}"))?;
    Ok(Argument::Text(text))
}

fn ping() -> Result<Argument, Error> {
    ui_message::message("pong")?;
    Ok(Argument::Null)
}

fn unregister() -> Result<Argument, Error> {
    editor_commands::unregister(COMMAND_ID)?;
    ui_message::message("unregistered")?;
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
