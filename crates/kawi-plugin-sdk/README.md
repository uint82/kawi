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

## Documentation

The authoring guide covers scaffolding, building, installing, granting, and troubleshooting:

https://github.com/uint82/kawi/blob/main/docs/help/plugin-authoring.md

The WIT contract is the source of truth for host methods, events, capabilities, and error codes.

## License

GPL-3.0-or-later
