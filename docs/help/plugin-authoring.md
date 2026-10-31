---
title = "Writing a WASM plugin"
aliases = ["wasm-plugin"]
related = ["plugins", "plugin-capabilities", "plugin-protocol", "plugin-migration"]
---
# Writing a WASM plugin

WASM plugins run in a WebAssembly component with explicit capabilities. This guide walks through creating, building, installing, granting, running, and inspecting a Rust plugin. See [Migrating a process plugin to WASM](help:plugin-migration) to port an existing JSON-RPC plugin.

The `kawi-plugin-sdk` crate re-exports the versioned WIT bindings and a small set of helpers. The release workflow publishes version 0.1.0 on crates.io. It pins `wit-bindgen` 0.63.0. The trim example and conformance suite build with that version and load the component under Wasmtime 50.0.1. A component that targets a different WIT version fails to load with the versioned import name. Other languages work if they target the component model, but only Rust has an SDK and tested template today.

## Prerequisites

Install the WebAssembly target:

```sh
rustup target add wasm32-wasip2
```

Install the component linker required by the WASM build:

```sh
cargo install wasm-component-ld --version 0.5.31 --locked
```

The `wasm` feature is enabled by default, so `:plugin inspect` and the runtime are available in a normal build:

```sh
cargo install --path crates/kawi
```

Build with `--no-default-features` only when you intentionally want to omit the WebAssembly runtime.

The scaffold uses the crates.io SDK by default. Pass `--sdk-path` only when developing against an unreleased checkout.

## Scaffold

```sh
kawi plugin new hello-plugin
```

The command writes a crate with:

- `Cargo.toml` using `kawi-plugin-sdk`
- `src/lib.rs` implementing the plugin lifecycle
- `plugin.toml` declaring runtime, component, capabilities, and commands
- `build.sh` wrapping the build and hash step
- `.gitignore` for the local target directory

The generated plugin exposes a `hello` command and requests no host capabilities.

## Build

```sh
kawi plugin build hello-plugin
```

The command runs:

```sh
cargo build --release --target wasm32-wasip2
```

It copies the component to the path in `plugin.toml`, computes SHA-256 over the component bytes, and rewrites `sha256`. Do not copy the hash by hand. A hand-copied hash is the most common authoring error.

`build.sh` is a thin wrapper around the same command.

## Install and grant

Copy the plugin directory into the Kawi plugin directory:

```sh
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins"
cp -r hello-plugin "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins/"
```

Start Kawi and inspect the manifest before granting anything:

```sh
kawi plugin inspect hello-plugin
```

The output lists the manifest requests and the component imports. Imports must be a subset of the requested capabilities. A component that imports `kawi:plugin/editor-read` must request `editor.read`, even if it never calls the import.

Grant capabilities through the prompt or explicitly:

```vim
:plugin grant hello-plugin editor.read
```

The grant is written to `$XDG_CONFIG_HOME/kawi/permissions.toml`. The approved request set is stored alongside the grant. An updated manifest that adds or widens a request reopens consent for only the changed capabilities.

## Run

The default plugin directory is `$XDG_CONFIG_HOME/kawi/plugins`. Plugin commands are namespaced by plugin name. The scaffold command is:

```vim
:hello-plugin.hello
```

For the bundled WASM trim example, run `:trim.trim` or the configured keybinding.

## Capability model from the author side

- `[capabilities]` in `plugin.toml` is the request. The host does not grant it automatically.
- `capabilities` on a command is an optional invocation gate. It must be a subset of `[capabilities]` and does not replace per-call authorization.
- The user grants, narrows, or denies each request in `permissions.toml`.
- A missing or corrupt approved record treats every request as new. The user sees the changed set in one prompt.
- A denied call returns `Error` with code `-32012`, a message, and the capability name. Handle it by reporting a useful message or stopping the operation.
- A plugin cannot use an import that it did not request. The host rejects the component before it runs.
- The default tier currently covers `ui.message`. Editor read, editor write, diagnostics publish, filesystem, process, and network require explicit grants.

## Contract details

- Positions use zero-based lines and UTF-8 byte columns, not character columns. A multibyte codepoint advances the byte column by its encoded length. Use `kawi_plugin_sdk::position_at` and `byte_offset` instead of counting characters.
- `editor.get_state` returns a revision. Pass that revision to `editor.apply_edit`. A stale revision returns code `-32010`.
- `kawi_plugin_sdk::apply_edit_with_retry` fetches fresh state, rebuilds edits, and retries a bounded number of times. Request `editor.read` and `editor.write` to use it.
- `buffer_changed` includes an origin. Ignore changes with origin `plugin:<name>` when your own edit would trigger a loop.
- The editor coalesces and orders events. A `file_saved` event is a good trigger for a full-buffer operation such as trim.
- Diagnostics accept at most 1000 items per buffer. Each message is at most 4096 bytes and each code is at most 256 bytes.
- The invocation budget defaults to 5 seconds and can be set from 100 ms to 60 seconds with `timeout_ms`. A timeout traps the call and counts toward restart backoff.
- WebAssembly threads are not enabled. Do not start threads or rely on shared memory.
- There is no ambient network access. `net.connect` is deny by default and WASI sockets are the only network surface.
- Network limits are 64 simultaneous TCP connections, 8 MiB inbound and 8 MiB outbound per connection, and 64 MiB shared per session. A session that exceeds a cap gets a short read or a closed write, not an unbounded allocation.
- WASI filesystem access is scoped by the plugin grants. The component can only see preopened paths inside granted scopes.

## Real edit example

`examples/wasm-trim` is the reference for a real edit. It reads state with `editor.get_state`, computes zero-based line and UTF-8 byte-column edits, and calls `apply_edit_with_retry`. The helper retries a stale revision twice, refuses to continue when the buffer identity changes, and returns `STALE_REVISION` after the retry budget. Report that error instead of applying the edits to a new buffer. The process trim example follows the same rules against the JSON-RPC contract.

## Troubleshooting

### The plugin requests a capability that is not granted

Run `kawi plugin inspect <dir>` and compare the `requests` lines with `permissions.toml`. Grant the missing capability with `:plugin grant <plugin> <capability>`.

### The component imports a capability that is not requested

The runtime refuses the component with a message naming the unauthorized import. Add the capability to `[capabilities]`, rebuild, and approve the new request.

### The component hash does not match

Run `kawi plugin build <dir>` again. Do not edit `sha256` manually. If the component is replaced after load, run `:plugin restart <name>` so the manifest is reread and the hash is checked again.

### The plugin traps or stays in backoff

Open `:plugin logs <name>`. A trap is reported with the WebAssembly error. Repeated traps increase restart backoff. Fix the trap rather than lowering the timeout.

### A grant is denied

A denied call returns `-32012`. Show the error through `ui.message` only if that capability is granted. Otherwise log a local message and return a clear plugin error.

## Honest scope

The Rust SDK and template are the supported authoring path. Any language that can produce a component targeting `kawi:plugin@0.1.0` can work, but it must provide its own bindings, UTF-8 position handling, stale-revision retry, and error mapping. The conformance suite is the source of truth for behavior across runtimes.
