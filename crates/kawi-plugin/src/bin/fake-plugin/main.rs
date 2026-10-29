use std::collections::VecDeque;
use std::io::{self, BufReader, Write};

use anyhow::Result;
use std::process::ExitCode;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use kawi_jsonrpc::framing::{read_message, write_message};

mod commands;

use commands::{
    apply_edit, apply_edit_mid_codepoint, control_text, diagnostics_call, exec_command, host_call,
    multi_call, read_file,
};

const HOST_STATE_REQUEST: u64 = 100;
static BUFFERED: Mutex<VecDeque<Value>> = Mutex::new(VecDeque::new());

fn main() -> ExitCode {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "normal".to_string());
    match run(&mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("fake plugin error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(mode: &str) -> Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::stdout().lock();
    while let Some(message) = read_message(&mut reader)? {
        if dispatch(mode, &message, &mut reader, &mut stdout)? {
            return Ok(());
        }
        drain_buffered(mode, &mut reader, &mut stdout)?;
    }
    Ok(())
}

fn dispatch(
    mode: &str,
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<bool> {
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return Ok(false);
    };
    match method {
        "initialize" => handle_initialize(mode, message, stdout)?,
        "invoke_command" => handle_invoke(mode, message, reader, stdout)?,
        "event" => handle_event(mode, message, reader, stdout)?,
        "shutdown" => return Ok(true),
        _ => {}
    }
    Ok(false)
}

fn drain_buffered(
    mode: &str,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    loop {
        let Some(message) = take_buffered() else {
            return Ok(());
        };
        let _ = dispatch(mode, &message, reader, stdout)?;
    }
}

pub(crate) fn buffer_message(message: Value) {
    BUFFERED
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push_back(message);
}

fn take_buffered() -> Option<Value> {
    BUFFERED
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .pop_front()
}

fn handle_initialize(mode: &str, message: &Value, stdout: &mut impl Write) -> Result<()> {
    match mode {
        "garbage" => {
            stdout.write_all(b"this is not json\n")?;
            stdout.flush()?;
            thread::sleep(Duration::from_secs(5));
        }
        "oversized" => {
            write!(stdout, "Content-Length: 17000000\r\n\r\n")?;
            stdout.flush()?;
            thread::sleep(Duration::from_secs(5));
        }
        "hang-init" => thread::sleep(Duration::from_secs(5)),
        "stalled" => {
            respond(stdout, message, initialize_result())?;
            thread::sleep(Duration::from_secs(5));
        }
        "stalled-events" => {
            respond(stdout, message, initialize_result())?;
            thread::sleep(Duration::from_secs(1));
        }
        _ => respond(stdout, message, initialize_result())?,
    }
    Ok(())
}

fn handle_event(
    mode: &str,
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    let Some(event) = message
        .get("params")
        .and_then(|params| params.get("event"))
        .and_then(Value::as_str)
    else {
        return Ok(());
    };
    match mode {
        "stalled-events" => eprintln!("event:{event}"),
        "event-host" => {
            let _ = host_request("ui.message", json!({"message": event}), reader, stdout)?;
        }
        "event-write" => {
            let _ = host_request(
                "ui.message",
                json!({"message": "event-write-start"}),
                reader,
                stdout,
            )?;
            let _ = host_request(
                "editor.apply_edit",
                json!({"buffer": 1, "revision": 7, "edits": []}),
                reader,
                stdout,
            )?;
            let _ = host_request(
                "ui.message",
                json!({"message": "event-write-done"}),
                reader,
                stdout,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_invoke(
    mode: &str,
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    match mode {
        "exit-mid" => std::process::exit(1),
        "crash-once" => {
            if marker_exists("crash-once.marker") {
                respond(stdout, message, json!({"echo": message.get("params")}))?;
            } else {
                create_marker("crash-once.marker")?;
                std::process::exit(1);
            }
        }
        "crash-five" => {
            let count = read_count("crash-five.marker")?;
            write_count("crash-five.marker", count + 1)?;
            if count < 5 {
                std::process::exit(1);
            }
            respond(stdout, message, json!({"echo": message.get("params")}))?;
        }
        "hang-invoke" => thread::sleep(Duration::from_secs(5)),
        "hang-invoke-once" => {
            if marker_exists("hang-invoke-once.marker") {
                respond(stdout, message, json!({"echo": message.get("params")}))?;
            } else {
                create_marker("hang-invoke-once.marker")?;
                thread::sleep(Duration::from_secs(5));
            }
        }
        "host-call" => host_call(message, reader, stdout)?,
        "multi-call" => multi_call(message, reader, stdout)?,
        "diagnostics" => diagnostics_call(message, reader, stdout)?,
        "apply-edit" => apply_edit(message, reader, stdout)?,
        "mid-codepoint" => apply_edit_mid_codepoint(message, reader, stdout)?,
        "control-text" => control_text(message, reader, stdout)?,
        "read-file" => read_file(message, stdout)?,
        "exec" => exec_command(message, stdout)?,
        _ => respond(stdout, message, json!({"echo": message.get("params")}))?,
    }
    Ok(())
}

fn host_request(
    method: &str,
    params: Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<Value> {
    write_message(
        stdout,
        &json!({
            "jsonrpc": "2.0",
            "id": HOST_STATE_REQUEST,
            "method": method,
            "params": params,
        }),
    )?;
    while let Some(value) = read_message(reader)? {
        if value.get("id").and_then(Value::as_u64) == Some(HOST_STATE_REQUEST) {
            return Ok(value);
        }
        if value.get("method").is_some() {
            buffer_message(value);
        }
    }
    Err(anyhow::anyhow!("host request ended without a response"))
}

fn marker_exists(name: &str) -> bool {
    std::path::Path::new(name).exists()
}

fn create_marker(name: &str) -> Result<()> {
    std::fs::write(name, b"")?;
    Ok(())
}

fn read_count(name: &str) -> Result<u32> {
    let text = std::fs::read_to_string(name).unwrap_or_else(|_| "0".to_string());
    Ok(text.trim().parse().unwrap_or(0))
}

fn write_count(name: &str, count: u32) -> Result<()> {
    std::fs::write(name, count.to_string())?;
    Ok(())
}

fn initialize_result() -> Value {
    json!({
        "protocol_version": 1,
        "name": "fake",
        "version": "0.1.0",
        "plugin_capabilities": ["invoke_command"],
        "host_capabilities": [
            "editor.get_state",
            "editor.apply_edit",
            "editor.set_diagnostics",
            "ui.message"
        ],
        "events": [
            "buffer_changed",
            "buffer_opened",
            "file_opened",
            "buffer_closed",
            "file_saved",
            "mode_changed",
            "diagnostics_changed"
        ],
    })
}

fn respond(stdout: &mut impl Write, request: &Value, result: Value) -> Result<()> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    write_message(
        stdout,
        &json!({"jsonrpc": "2.0", "id": id, "result": result}),
    )
}

fn respond_error(stdout: &mut impl Write, request: &Value, error: Value) -> Result<()> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    write_message(stdout, &json!({"jsonrpc": "2.0", "id": id, "error": error}))
}
