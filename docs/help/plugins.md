---
title = "Plugins"
aliases = ["plugin"]
related = ["plugin-protocol", "keymaps", "configuration", "troubleshooting"]
---
# Plugins

Kawi supports process plugins that communicate over JSON-RPC 2.0. Plugins are trusted code and run with your full user permissions.

Kawi loads plugins only from `$XDG_CONFIG_HOME/kawi/plugins`, normally `~/.config/kawi/plugins`. It never loads plugins from a project directory.

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
- Commands and default keybindings are loaded from the manifest at startup. No process starts until an invocation or activation event.
- Plugin keybindings have lower precedence than user mappings. They survive `:map reload`.
- Count and register prefixes are reserved and cannot start a plugin binding.

## Commands

- `:plugin list` shows installed plugins, versions, status, and commands.
- `:plugin restart` restarts every plugin process.
- `:plugin restart example` restarts one plugin and clears its crash backoff.
- `:plugin logs` shows all plugin logs.
- `:plugin logs example` shows one plugin's log.

Use `:commands` to discover plugin command names. Plugin commands appear with a `[plugin: name]` marker.

## Lifecycle

1. Kawi reads manifests and registers static commands and keybindings.
2. The first invocation or activation event starts the plugin process.
3. Kawi performs capability negotiation with `initialize`.
4. Kawi sends `invoke_command` or events.
5. A crashed plugin restarts on the next invocation with exponential backoff. Five consecutive failures disable it until `:plugin restart`.

Kawi supports one in-flight invocation per plugin. A plugin may call back into the host while an invocation is running. Events continue to flow during an invocation. A timed-out invocation is canceled and the process is restarted.

## Events

Kawi sends `buffer_changed`, `buffer_opened`, `file_opened`, `buffer_closed`, `file_saved`, `mode_changed`, and `diagnostics_changed`. Events are notifications and never require a response.

- Delivery is separate from activation. A stopped plugin receives nothing and is not started by an event.
- Each plugin has a bounded queue. Buffer changes are coalesced per buffer, optional debounce applies, and other events are dropped with a log line when the queue is full.
- `buffer_changed` carries the buffer identifier, revision, previous revision, origin, UTF-8 byte edits, and a `full_resync` flag. Apply edits in order, or call `editor.get_state` when the revision does not match.
- Origins are `user`, `undo`, `plugin:<name>`, and `external_reload`. A plugin does not receive its own edits.
- `:plugin logs example` reports queue overflows.

Kawi sends diagnostics from LSP and tasks to plugins. Plugin-published diagnostics are reserved for a future minor version.

## Host methods

- `editor.get_state` returns the buffer identifier, revision, path, language, mode, cursor, selection, and optionally the full text.
- `editor.apply_edit` applies text edits only when the supplied buffer identifier and revision match the current buffer. Stale edits are rejected and the buffer is unchanged.
- `ui.message` shows a status message.

Positions are zero-based and use line plus UTF-8 byte column. Kawi rejects byte columns inside a code point.

See [Plugin protocol](help:plugin-protocol) for framing, capability negotiation, JSON shapes, and error codes.

## Example

The repository includes two Python examples:

- `examples/plugins/echo` registers `echo.ping` and `echo.hello`.
- `examples/plugins/trim` trims trailing whitespace on `file_saved` and through the `trim.trim` command.

Copy either directory to `~/.config/kawi/plugins` to try it.

Standard output is protocol-only. Write logs to standard error and read them with `:plugin logs`.
