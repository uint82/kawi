# kawi-plugin-sdk

Rust SDK for Kawi WebAssembly plugins. It re-exports the versioned `kawi:plugin@0.1.0` WIT bindings and provides helpers for UTF-8 positions, stale-revision retries, and protocol error codes.

## Install

```toml
[dependencies]
kawi-plugin-sdk = "0.1.0"
```

The SDK requires Rust 1.98 or newer and the `wasm32-wasip2` target.

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

## Documentation

The authoring guide covers scaffolding, building, installing, releasing, and troubleshooting:

https://github.com/uint82/kawi/blob/main/docs/help/plugin-authoring.md

The WIT contract is the source of truth for host methods, events, and error codes. Capability names and scopes are defined in the authoring guide and the plugin capability reference.

## License

GPL-3.0-or-later
