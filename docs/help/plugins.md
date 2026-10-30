---
title = "Plugins"
aliases = ["plugin"]
related = ["plugin-protocol", "plugin-capabilities", "plugin-authoring", "keymaps", "configuration", "troubleshooting"]
---
# Plugins

Kawi supports process plugins that communicate over JSON-RPC 2.0 and WebAssembly plugins that use a versioned component contract. Process plugins are trusted code and run with your full user permissions. WASM plugins are the restricted path and are the recommended authoring target. See [Writing a WASM plugin](help:plugin-authoring).

Kawi loads plugins only from `$XDG_CONFIG_HOME/kawi/plugins`, normally `~/.config/kawi/plugins`. It never loads plugins from a project directory.

Plugins request capabilities in their manifest, and users grant them in `$XDG_CONFIG_HOME/kawi/permissions.toml`. See [Plugin capabilities](help:plugin-capabilities). Denials open a consent prompt. On Linux with a full Landlock ABI, Landlock enforces filesystem and process-execution scopes. A feature-gated WebAssembly runtime executes components with a deny-by-default WIT contract, manifest-bound component hashes, WASI preopens from capability scopes, lifecycle calls, editor host calls, events, authorized process execution, wall-clock call limits, and cancellation of blocked host calls on restart. Start `kawi --no-plugins` to skip plugin discovery, hashing, and WebAssembly compilation entirely. WebAssembly network scopes are enforced with DNS pinning, filtered guest name lookup, and non-public address rejection. Process network scopes and older ABI filesystem scopes remain advisory.

Each plugin lives in its own directory:

```text
~/.config/kawi/plugins/example/plugin.toml
~/.config/kawi/plugins/example/plugin.py
```

## Manifest

```toml
name = "example"
version = "0.1.0"
protocol = 1
description = "Example plugin"
command = ["python3", "${plugin_dir}/plugin.py"]
timeout_ms = 5000
subscribes = ["buffer_changed", "file_saved"]

[debounce]
buffer_changed = 100

[[activation_events]]
event = "on_command"

[[activation_events]]
event = "on_file_open"
filetypes = ["rust"]
globs = ["*.rs"]

[[commands]]
id = "format"
title = "Format buffer"
description = "Format the current buffer"

[commands.keybindings]
normal = ["<leader>pf"]
```

The plugin name is the namespace. The command above is `example.format`, and you can bind it in `keymaps.toml` like any built-in command.

- `command` is an executable and its arguments. `${plugin_dir}` resolves to the plugin directory, which is also the working directory.
- `timeout_ms` applies to initialization and each invocation. The default is `5000`. The range is `100` to `60000`.
- `subscribes` lists events delivered to a running plugin. Delivery never starts a plugin.
- `debounce` optionally delays an event per subscription. `buffer_changed` is also coalesced per buffer.
- `activation_events` lists `on_startup`, `on_command`, and `on_file_open` with optional `filetypes` and `globs`.
- Commands and default keybindings are loaded from the manifest at startup and whenever `:plugin restart` reloads manifests. No process starts until an invocation or activation event.
- Plugin keybindings have lower precedence than user mappings. They survive `:map reload`.
- Count and register prefixes are reserved and cannot start a plugin binding.

## Commands

- `:plugin list` shows installed plugins, versions, status, effective capabilities, and commands.
- `:plugin grant <plugin> <capability>` opens the consent prompt for a requested capability.
- `:plugin reload` reloads `permissions.toml` and applies revocations to the next host call or event. Filesystem and execute scope changes restart the affected plugin so Landlock re-applies. Kawi also reloads the file automatically after a debounced external save.
- `:plugin restart` reloads every manifest, reconciles commands and keybindings, and restarts every plugin.
- `:plugin restart example` reloads one manifest, reconciles its commands and keybindings, restarts that plugin, and clears its crash backoff.
- `:plugin logs` shows all plugin logs.
- `:plugin inspect <plugin>` shows the runtime, computed and declared component hashes, request hash, requested capabilities, and WASM imports when the `wasm` feature is enabled.
- `:plugin logs example` shows one plugin's log.

Use `:commands` to discover plugin command names. Plugin commands appear with a `[plugin: name]` marker.

## Lifecycle

1. Kawi reads manifests and registers static commands and keybindings.
2. `:plugin restart` rereads manifests, reconciles commands and keybindings, and restarts the selected plugin or all plugins.
3. The first invocation or activation event starts the plugin process.
4. Kawi performs capability negotiation with `initialize`.
5. Kawi sends `invoke_command` or events.
6. A crashed plugin restarts on the next invocation with exponential backoff. Five consecutive failures disable it until `:plugin restart`.

Kawi supports one in-flight invocation per plugin. A plugin may call back into the host while an invocation is running. Events continue to flow during an invocation. A timed-out invocation is canceled and the process is restarted.

## Events

Kawi sends `buffer_changed`, `buffer_opened`, `file_opened`, `buffer_closed`, `file_saved`, `mode_changed`, and `diagnostics_changed`. Events are notifications and never require a response.

- Delivery is separate from activation. A stopped plugin receives nothing and is not started by an event.
- Each plugin has a bounded queue. Buffer changes are coalesced per buffer, optional debounce applies, and other events are dropped with a log line when the queue is full.
- `buffer_changed` carries the buffer identifier, revision, previous revision, origin, UTF-8 byte edits, and a `full_resync` flag. Apply edits in order, or call `editor.get_state` when the revision does not match.
- Origins are `user`, `undo`, `plugin:<name>`, `external_reload`, and `mixed`. A plugin does not receive its own edits. Mixed origins force a resync instead of partial filtering.
- `:plugin logs example` reports queue overflows. An overflow forces a resync on the next delivered change. Crashes and restarts clear queued events.

Kawi sends diagnostics from LSP and tasks to plugins. A plugin with `diagnostics.publish` can publish diagnostics for the current named buffer through `editor.set_diagnostics`. Kawi prefixes the source with the plugin name, so plugins cannot spoof another provider. Publishing an empty list clears that plugin's diagnostics for the buffer.

## Host methods

- `editor.get_state` returns the buffer identifier, revision, path, language, mode, cursor, selection, and optionally the full text.
- `editor.apply_edit` applies text edits only when the supplied buffer identifier and revision match the current buffer. Stale edits are rejected and the buffer is unchanged.
- `editor.set_diagnostics` replaces the plugin-published diagnostics for the current buffer. The buffer identifier and revision must match, and the buffer must have a file path.
- `ui.message` shows a status message.

Positions are zero-based and use line plus UTF-8 byte column. Kawi rejects byte columns inside a code point.

See [Plugin protocol](help:plugin-protocol) for framing, capability negotiation, JSON shapes, and error codes.

## Example

The repository includes two Python examples:

- `examples/plugins/echo` registers `echo.ping` and `echo.hello`.
- `examples/plugins/trim` trims trailing whitespace after `file_saved` and through the `trim.trim` command. It is a trim-after-save plugin: the saved file is not trimmed until the next save, and the buffer becomes dirty after trimming. It is not a format-before-save hook.

Copy either directory to `~/.config/kawi/plugins` to try it.

Standard output is protocol-only. Write logs to standard error and read them with `:plugin logs`.
