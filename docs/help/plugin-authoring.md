---
title = "Writing a WASM plugin"
aliases = ["wasm-plugin"]
related = ["plugins", "plugin-capabilities", "plugin-protocol", "plugin-migration"]
---
# Writing a WASM plugin

WASM plugins run in a WebAssembly component with a versioned manifest. This guide covers creating, building, publishing, installing, running, and inspecting a Rust plugin. See [Migrating a process plugin to WASM](help:plugin-migration) to port an existing JSON-RPC plugin.

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
- `plugin.toml` declaring runtime, component, timeout, activation events, and commands
- `build.sh` wrapping the build and hash step
- `.gitignore` for the local target directory

The generated plugin exposes a `hello` command and declares no host APIs.

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

## Publish a release

Managed installation expects a GitHub release. The release commit must contain `plugin.toml` at the repository root. The release must include the component as an asset whose file name matches the manifest `component` file name.

```toml
name = "hello-plugin"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "hello-plugin.wasm"
sha256 = "sha256:..."
```

Attach `hello-plugin.wasm` to the release. Kawi downloads that asset, checks it against `sha256`, and validates the component imports before installing.

A plugin author does not need to publish a registry entry. The repository is the package identity, and the release tag or commit is the version.

## Install

For a released plugin:

```vim
:plugin install owner/repo
:plugin install owner/repo@v0.1.0
```

Kawi resolves the release, reads the manifest, checks the API version, and shows one confirmation with the repository, author, commit, API version, checksum, and declared host APIs. Enter installs and enables the plugin. The commit and checksum are pinned in `$XDG_CONFIG_HOME/kawi/plugins.lock`.

There is no grant step. Installing the plugin is the trust decision. Declared host APIs are enabled automatically.

## Develop locally

Copy the plugin directory into the development plugin directory:

```sh
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins"
cp -r hello-plugin "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins/"
```

Start Kawi. It prints a development plugin warning and marks the plugin as `development` in `:plugin list`.

Inspect the manifest and component imports before running it:

```sh
kawi plugin inspect "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins/hello-plugin"
```

The output lists the declared host APIs and the component imports. Imports must be a subset of the declarations. A component that imports `kawi:plugin/editor-read` must declare `editor.read`, even if it never calls the import.

The default development plugin directory is `$XDG_CONFIG_HOME/kawi/plugins`. Plugin commands are namespaced by plugin name. The scaffold command is:

```vim
:hello-plugin.hello
```

For the bundled WASM trim example, run `:trim.trim` or the configured keybinding.

Development plugins are not managed by `:plugin install`, `:plugin update`, `:plugin disable`, or `:plugin remove`. Edit or remove them directly.

## Host API declarations

- `[capabilities]` in `plugin.toml` declares the host APIs the component uses. A component must declare every interface it imports, including `ui.message`.
- Kawi enables every declared API for a managed plugin. Declarations are a compatibility and audit surface, not a per-capability grant.
- `capabilities` on a command is an optional invocation gate. It must be a subset of `[capabilities]`.
- `editor.execute` is scoped by an explicit `commands` allowlist. Declare it with `"editor.execute" = { commands = ["edit.format", "file.save"] }`.
- A component that imports an undeclared interface is rejected before it runs.
- A denied host call returns `Error` with code `-32012`, a message, and the API name. Handle it by reporting a useful message or stopping the operation.
- On update, Kawi prompts only when the declared API surface grows. The prompt shows the new commit and the added or widened APIs.

## Contract details

- Positions use zero-based lines and UTF-8 byte columns, not character columns. A multibyte codepoint advances the byte column by its encoded length. Use `kawi_plugin_sdk::position_at` and `byte_offset` instead of counting characters.
- `editor.get_state` returns a revision. Pass that revision to `editor.apply_edit`. A stale revision returns code `-32010`.
- `kawi_plugin_sdk::apply_edit_with_retry` fetches fresh state, rebuilds edits, and retries a bounded number of times. Declare `editor.read` and `editor.write` to use it.
- `editor_execute::list_commands` returns only commands in the allowlist. `editor_execute::execute` runs a canonical built-in command. Plugin-defined commands, `keys.send`, and commands that require arguments are rejected. The editor runs the command before the host call returns.
- `buffer_changed` includes an origin. Kawi excludes your own edits, so do not filter by origin to avoid loops. A `mixed` origin includes your own edit or changes from several sources; resync when `full_resync` is true.
- The editor coalesces and orders events. A `file_saved` event is a good trigger for a full-buffer operation such as trim.
- Diagnostics accept at most 1000 items per buffer. Each message is at most 4096 bytes and each code is at most 256 bytes.
- The invocation budget defaults to 5 seconds and can be set from 100 ms to 60 seconds with `timeout_ms`. A timeout traps the call and counts toward restart backoff.
- WebAssembly threads are not enabled. Do not start threads or rely on shared memory.
- There is no ambient network access. `net.connect` is deny by default and WASI sockets are the only network surface.
- Network limits are 64 simultaneous TCP connections, 8 MiB inbound and 8 MiB outbound per connection, and 64 MiB shared per session. A session that exceeds a cap gets a short read or a closed write, not an unbounded allocation.
- WASI filesystem access is scoped by declared `fs.read` and `fs.write` paths. The component can only see preopened paths inside those scopes. Preopens are directories; a file path is skipped with a log line.

## Real edit example

`examples/wasm-trim` is the reference for a real edit. It reads state with `editor.get_state`, computes zero-based line and UTF-8 byte-column edits, and calls `apply_edit_with_retry`. The helper retries a stale revision twice, refuses to continue when the buffer identity changes, and returns `STALE_REVISION` after the retry budget. Report that error instead of applying the edits to a new buffer. The process trim example follows the same rules against the JSON-RPC contract.

## Troubleshooting

### The component imports an API that is not declared

The runtime refuses the component with a message naming the unauthorized import. Add the API to `[capabilities]`, rebuild, and publish a new release.

### The component hash does not match

Run `kawi plugin build <dir>` again. Do not edit `sha256` manually. For a managed plugin, publish a new release and run `:plugin update`. For a development plugin, run `:plugin restart <name>`.

### The plugin traps or stays in backoff

Open `:plugin logs <name>`. A trap is reported with the WebAssembly error. Repeated traps increase restart backoff. Fix the trap rather than lowering the timeout.

### A host call is denied

A denied call returns `-32012`. The component tried to use an interface it did not declare or a host method outside the declared API. Add the declaration and publish a new release, or remove the call.

## Honest scope

The Rust SDK and template are the supported authoring path. Any language that can produce a component targeting `kawi:plugin@0.1.0` can work, but it must provide its own bindings, UTF-8 position handling, stale-revision retry, and error mapping. The conformance suite is the source of truth for behavior across runtimes.
