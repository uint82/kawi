use std::io::{self, BufReader, Write};

use anyhow::Result;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use kawi_jsonrpc::framing::{read_message, write_message};

const HOST_STATE_REQUEST: u64 = 100;

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
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            continue;
        };
        match method {
            "initialize" => handle_initialize(mode, &message, &mut stdout)?,
            "invoke_command" => handle_invoke(mode, &message, &mut reader, &mut stdout)?,
            "shutdown" => return Ok(()),
            _ => {}
        }
    }
    Ok(())
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
        _ => respond(stdout, message, initialize_result())?,
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
        _ => respond(stdout, message, json!({"echo": message.get("params")}))?,
    }
    Ok(())
}

fn host_call(
    message: &Value,
    reader: &mut impl std::io::BufRead,
    stdout: &mut impl Write,
) -> Result<()> {
    write_message(
        stdout,
        &json!({
            "jsonrpc": "2.0",
            "id": HOST_STATE_REQUEST,
            "method": "editor.get_state",
            "params": {"include_text": true},
        }),
    )?;
    while let Some(value) = read_message(reader)? {
        if value.get("id").and_then(Value::as_u64) != Some(HOST_STATE_REQUEST) {
            continue;
        }
        return respond(
            stdout,
            message,
            json!({
                "host_result": value.get("result").cloned().unwrap_or(Value::Null),
                "host_error": value.get("error").cloned().unwrap_or(Value::Null),
            }),
        );
    }
    Ok(())
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
        "host_capabilities": ["editor.get_state", "editor.apply_edit", "ui.message"],
        "events": ["buffer_changed", "file_saved", "mode_changed"],
    })
}

fn respond(stdout: &mut impl Write, request: &Value, result: Value) -> Result<()> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    write_message(
        stdout,
        &json!({"jsonrpc": "2.0", "id": id, "result": result}),
    )
}
