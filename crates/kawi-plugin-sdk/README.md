# kawi-plugin-sdk

Rust SDK for Kawi WebAssembly plugins. It re-exports the versioned `kawi:plugin@0.8.0` WIT bindings and provides helpers for UTF-8 positions, stale-revision retries, and protocol error codes.

## Install

```toml
[dependencies]
kawi-plugin-sdk = "0.8.0"
```

The SDK requires Rust 1.98 or newer and the `wasm32-wasip2` target. Components built against an earlier `kawi:plugin` package must be rebuilt because the WIT import namespace is versioned.

## Example

```rust
use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, Error, Event};
use kawi_plugin_sdk::INVALID_PARAMS;

const COMMAND_ID: &str = "hello";

struct Plugin;

impl Guest for Plugin {
    fn init(_host_protocol: u32) -> Result<(), Error> {
        Ok(())
    }

    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {
        if id != COMMAND_ID {
            return Err(Error {
                code: INVALID_PARAMS,
                message: "unknown command".to_string(),
                capability: None,
            });
        }
        Ok(Argument::Text("hello".to_string()))
    }

    fn on_event(_event: Event) {}
}

kawi_plugin_sdk::export!(Plugin);
```

## Composition

Declare an explicit command allowlist in `plugin.toml`:

```toml
[capabilities]
"editor.execute" = { commands = ["edit.format", "file.save"] }
```

The `editor_execute` module exposes `execute` and `list_commands`. The allowlist uses canonical command IDs, and the host checks every call before running the command.

## Settings

Declare plugin settings in `plugin.toml`:

```toml
[settings.max_items]
type = "integer"
default = 10
minimum = 1
maximum = 100

[settings.mode]
type = "string"
default = "auto"
enum = ["auto", "light", "dark"]
```

Users override them in `$XDG_CONFIG_HOME/kawi/plugins.toml`:

```toml
[example]
max_items = 25
mode = "dark"
```

The `config_read` module exposes `get` and `get_all`. Settings are namespaced to the plugin and are available without a capability declaration. Invalid values fall back to the manifest default and are logged. The host reads the file on demand, so changes apply without restarting the plugin.

## Interactive UI

Declare the matching capabilities in `plugin.toml`:

```toml
[capabilities]
"ui.prompt" = true
"ui.progress" = true
```

The `ui_prompt` module exposes `input`, `select`, and `confirm`. Each call blocks until the user responds and returns a prompt outcome:

```rust
use kawi_plugin_sdk::types::PromptOutcome;
use kawi_plugin_sdk::ui_prompt;

let outcome = ui_prompt::confirm("Delete file?", None)?;
if outcome == PromptOutcome::Confirmed(true) {
}
```

The `ui_progress` module exposes `start`, `update`, and `finish`. `update` returns `true` after the user cancels the operation. Call `finish` for every started progress. Prompt calls suspend the invocation timeout until the user responds. Progress calls remain bounded by it.

## Panels

Declare `ui.panel` in `plugin.toml` and use `ui_panel` to publish a read-only text panel:

```rust
use kawi_plugin_sdk::ui_panel;

let panel = ui_panel::open(&ui_panel::OpenParams {
    title: "Index".to_string(),
    status: Some("building".to_string()),
    height: Some(8),
    focus: true,
    lines: vec![ui_panel::PanelLine {
        text: "src/main.rs".to_string(),
        style: None,
    }],
})?;
ui_panel::update(&ui_panel::UpdateParams {
    panel: panel.panel,
    title: None,
    status: Some("complete".to_string()),
    height: None,
    lines: None,
})?;
ui_panel::close(panel.panel)?;
```

Only one panel can be active across all plugins. A second open returns `UI_BUSY`. Each plugin may update only its own panel. The host owns focus, layout, scrolling, and closing. Panel content is limited to 1000 lines, 4096 bytes per line, 1 MiB total, a 256-byte title, and a 256-byte status. There is no raw key input API.

## Process jobs

Declare `process.spawn` in `plugin.toml` and use `process_jobs` to open long-running jobs:

```rust
use kawi_plugin_sdk::process_jobs::{self, JobEvent, OpenParams, StdinMode};

let job = process_jobs::open(&OpenParams {
    command: "git".to_string(),
    args: vec!["status".to_string()],
    cwd: None,
    env: Vec::new(),
    stdin: StdinMode::Null,
})?;

for event in process_jobs::poll(job)? {
    match event {
        JobEvent::Output(output) => println!("{}", output.data),
        JobEvent::Exited(exited) => println!("exited {:?}", exited.exit_code),
    }
}
```

The `process_jobs` module exposes `open`, `write`, `close_stdin`, `signal`, `kill`, `status`, and `poll`. WASM plugins are passive, so the host queues job output and exit events and `poll` drains them on the next plugin call. Process plugins receive the same events through the JSON-RPC event stream.

## Visual decorations

Declare decoration groups in `plugin.toml`:

```toml
[[decorations]]
id = "todo"
color = "info"
fill = "both"
italic = true

[[decorations]]
id = "todo_text"
color = "info"
fill = "foreground"
italic = true
```

Highlights, virtual text, and signs reference a group by name. Themes override groups with `[plugin_decorations.<plugin>.<group>]`. Group IDs are local to the plugin and may not use the built-in style names.

## Documentation

The authoring guide covers scaffolding, building, installing, releasing, and troubleshooting:

https://github.com/uint82/kawi/blob/main/docs/help/plugin-authoring.md

The WIT contract is the source of truth for host methods, events, and error codes. Capability names and scopes are defined in the authoring guide and the plugin capability reference.

## License

GPL-3.0-or-later
