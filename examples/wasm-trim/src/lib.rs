use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, Edit, Error, Event, Position};
use kawi_plugin_sdk::{
    apply_edit_with_retry, editor_read, ui_message, INTERNAL_ERROR, INVALID_PARAMS,
};

const PROTOCOL_VERSION: u32 = 1;
const COMMAND_ID: &str = "trim";

struct Trim;

impl Guest for Trim {
    fn init(host_protocol: u32) -> Result<(), Error> {
        if host_protocol == PROTOCOL_VERSION {
            Ok(())
        } else {
            Err(plugin_error(
                INVALID_PARAMS,
                "unsupported host protocol version",
            ))
        }
    }

    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {
        if id != COMMAND_ID {
            return Err(plugin_error(INVALID_PARAMS, "unknown command"));
        }
        trim_buffer().map(Argument::Text)
    }

    fn on_event(event: Event) {
        if matches!(event, Event::FileSaved(_)) {
            let _ = trim_buffer();
        }
    }
}

fn trim_buffer() -> Result<String, Error> {
    let result = trim_state();
    if let Err(error) = &result {
        let _ = ui_message::message(&error.message);
    }
    result
}

fn trim_state() -> Result<String, Error> {
    let state = editor_read::get_state(true)?;
    let text = state.text.as_deref().unwrap_or_default();
    let edits = trim_edits(text)?;
    if edits.is_empty() {
        return Ok("no trailing whitespace".to_string());
    }
    let count = edits.len();
    apply_edit_with_retry(state.buffer, state.revision, edits, |state| {
        trim_edits(state.text.as_deref().unwrap_or_default())
    })?;
    Ok(format!("trimmed {count} lines"))
}

fn trim_edits(text: &str) -> Result<Vec<Edit>, Error> {
    let mut edits = Vec::new();
    for (line, raw) in text.split('\n').enumerate() {
        let trimmed = raw.trim_end_matches([' ', '\t']);
        if trimmed.len() == raw.len() {
            continue;
        }
        edits.push(Edit {
            start: position(line, trimmed.len())?,
            end: position(line, raw.len())?,
            text: String::new(),
        });
    }
    Ok(edits)
}

fn position(line: usize, byte_column: usize) -> Result<Position, Error> {
    Ok(Position {
        line: u32::try_from(line).map_err(|_| plugin_error(INTERNAL_ERROR, "line exceeds u32"))?,
        byte_column: u32::try_from(byte_column)
            .map_err(|_| plugin_error(INTERNAL_ERROR, "byte column exceeds u32"))?,
    })
}

fn plugin_error(code: i32, message: &str) -> Error {
    Error {
        code,
        message: message.to_string(),
        capability: None,
    }
}

kawi_plugin_sdk::export!(Trim);
