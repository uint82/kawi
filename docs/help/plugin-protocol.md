---
title = "Plugin protocol"
aliases = ["plugin-protocol-v1"]
related = ["plugins", "plugin-capabilities", "plugin-authoring", "plugin-migration", "configuration", "keymaps"]
---
# Plugin protocol

This document defines Kawi plugin protocol version 1. Minor version 1 adds events. Minor version 2 adds plugin-published diagnostics. Minor version 3 adds editor command composition. Minor version 4 adds plugin settings. Minor version 7 adds buffer decorations and status line segments. Minor version 8 adds decoration scopes. Minor version 9 adds named decoration groups. A plugin that implements an earlier minor version still initializes and receives its declared events; it simply does not use the newer host methods. The protocol is a JSON-RPC 2.0 contract between Kawi and an external process. WebAssembly plugins implement the same behavior through the versioned WIT contract described in [Migrating a process plugin to WASM](help:plugin-migration).

## Trust model

Process plugins run as your user in development mode. Kawi loads them only from `$XDG_CONFIG_HOME/kawi/plugins`, never from a project directory. Install only plugins you trust. Host-mediated editor, UI, and diagnostics access is scoped by the manifest declarations. On Linux with a full Landlock ABI, a ruleset also limits filesystem reads, writes, and executable paths to declared scopes. Managed installs use WASM; see [Plugin host APIs](help:plugin-capabilities).

Each plugin lives in its own directory:

```text
~/.config/kawi/plugins/example/plugin.toml
~/.config/kawi/plugins/example/plugin.py
```

Relative `${plugin_dir}` references resolve to the plugin directory. The process working directory is the plugin directory.

## Manifest

```toml
name = "example"
version = "0.1.0"
protocol = 1
description = "Example plugin"
command = ["python3", "${plugin_dir}/plugin.py"]
timeout_ms = 5000
subscribes = ["buffer_changed", "file_saved"]

[settings.max_items]
type = "integer"
default = 10
minimum = 1
maximum = 100

[debounce]
buffer_changed = 100

[[activation_events]]
event = "on_file_open"
filetypes = ["rust"]
globs = ["*.rs"]

[[commands]]
id = "ping"
title = "Ping"
description = "Return a greeting"

[commands.keybindings]
normal = ["<leader>pp"]
```

The namespace is the plugin `name`. The command above is invoked as `example.ping` and can be bound in `keymaps.toml`.

`timeout_ms` applies to initialization and each command invocation. The default is `5000`. The accepted range is `100` to `60000`.

`subscribes` lists events delivered to a running plugin. Delivery never starts a plugin. `debounce` optionally delays an event per subscription. `activation_events` lists events that may start the plugin:

- `on_startup`
- `on_command`
- `on_file_open` with optional `filetypes` and `globs`

Unknown event names and unknown fields are ignored.

## Transport

Messages use LSP-style framing:

```text
Content-Length: <bytes>\r\n
\r\n
<UTF-8 JSON body>
```

- Standard output is protocol-only. Plugins must write logs to standard error.
- Standard error is line-oriented log output.
- The maximum message size is 16 MiB.
- Host responses larger than the maximum message size are rejected with `-32603`.
- Malformed JSON, invalid framing, or oversized messages terminate the process.
- Request IDs are JSON-RPC IDs. Host requests use integer IDs.

## Lifecycle

1. Kawi reads the manifest and registers static commands and default keybindings. No process is started.
2. The first command invocation starts the process.
3. Kawi sends `initialize` and waits for the response.
4. Kawi sends `invoke_command`.
5. The process exits when Kawi shuts down or the host sends `shutdown`.

Kawi supports one in-flight invocation per plugin. Calls from the plugin to the host may happen while an invocation is running. Prompt calls suspend the invocation timeout until the user responds. Other calls, including progress updates, remain bounded by the timeout. A timed-out invocation sends `$/cancelRequest` and restarts the process.

## Initialize

Host request:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocol_version": 1,
    "host": {"name": "kawi", "version": "0.0.1"},
    "position_encoding": "utf-8-bytes",
    "protocol_minor": 9,
    "host_capabilities": ["editor.get_state", "editor.apply_edit", "editor.execute", "editor.list_commands", "editor.set_diagnostics", "editor.set_decorations", "ui.set_statusline", "ui.message", "ui.input", "ui.select", "ui.confirm", "ui.progress_start", "ui.progress_update", "ui.progress_finish", "config.get", "config.list"],
    "plugin_capabilities": ["invoke_command"],
    "events": ["buffer_changed", "buffer_opened", "file_opened", "buffer_closed", "file_saved", "mode_changed", "diagnostics_changed"]
  }
}
```

Plugin response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocol_version": 1,
    "protocol_minor": 9,
    "name": "example",
    "version": "0.1.0",
    "plugin_capabilities": ["invoke_command"],
    "host_capabilities": ["editor.get_state", "editor.apply_edit", "editor.execute", "editor.list_commands", "editor.set_diagnostics", "editor.set_decorations", "ui.set_statusline", "ui.message", "ui.input", "ui.select", "ui.confirm", "ui.progress_start", "ui.progress_update", "ui.progress_finish", "config.get", "config.list"],
    "events": ["buffer_changed", "file_saved"]
  }
}
```

Initialization fails when the protocol version differs, `invoke_command` is not declared, or the plugin declares an unknown host capability. Calling an unknown method returns `-32601`; a known method outside the declared host APIs returns `-32012`.

Kawi sends only events present in both the manifest `subscribes` list and the plugin `events` list. A plugin that returns no `events` receives no notifications.

## Invoke command

Host request:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "invoke_command",
  "params": {
    "command": "ping",
    "args": null,
    "context": {
      "buffer": 1,
      "revision": 12,
      "path": "/home/user/project/src/main.rs",
      "language": "rust",
      "mode": "normal",
      "cursor": {"line": 4, "byte_column": 8},
      "selection": null
    }
  }
}
```

`args` is the binding argument value or `null`. `path` and `language` may be `null`. `selection` is `null` or `{"start": position, "end": position}`.

The result is any JSON value. Kawi does not display it automatically. Use `ui.message` for user-visible output.

## Events

Kawi sends notifications with method `event`:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "event": "buffer_changed",
    "payload": {
      "buffer": 1,
      "revision": 14,
      "previous_revision": 12,
      "origin": "user",
      "full_resync": false,
      "changes": [
        {
          "start": {"line": 4, "byte_column": 8},
          "end": {"line": 4, "byte_column": 12},
          "text": "value"
        }
      ]
    }
  }
}
```

Event names:

- `buffer_changed`
- `buffer_opened`
- `file_opened`
- `buffer_closed`
- `file_saved`
- `mode_changed`
- `diagnostics_changed`

`buffer_opened` reports that a buffer exists. `file_opened` reports that a buffer was loaded from a path. Scratch buffers have `path: null`. `mode_changed` carries `from` and `to` using the same names as keymap configuration. `buffer_closed` lets plugins release per-buffer state.

`buffer_changed` carries revisions, not full text. Apply `changes` in order. If `previous_revision` does not match the last revision the plugin applied, or `full_resync` is true, call `editor.get_state`. Kawi coalesces rapid changes and may mark a coalesced event `full_resync`.

`origin` is one of `user`, `undo`, `plugin:<name>`, `external_reload`, or `mixed`. A plugin does not receive its own `plugin:<name>` edits by default. When origins or own edits mix, Kawi sends `full_resync` instead of filtering part of the change.

`diagnostics_changed` carries `buffer`, `path`, `revision`, and a `diagnostics` array. Each diagnostic has `start`, `end`, `severity`, `message`, `source`, and `code`. `revision` is the diagnostics revision, not the buffer revision.

Events are post-hoc notifications. They never wait for a plugin response. Kawi does not implement `will_save` in version 1. Use `file_saved` and invoke a command for format-on-save behavior.

Notifications and requests share one ordered pipe per plugin. Ordering is guaranteed per plugin and per event type. `editor.get_state` always returns live state and includes the current revision.

When a plugin stops, queued events are dropped. An event already being handled may finish, including any host call already in flight.

Each plugin has a bounded event queue. Kawi coalesces or debounces `buffer_changed`; other events are dropped with a log line when the queue is full. An overflow marks the buffer for resync, so the next delivered change has `full_resync: true`. A crash or restart clears queued events and forces resync.

Kawi sends diagnostics to plugins and accepts plugin-published diagnostics through `editor.set_diagnostics`, scoped by plugin name.

## Positions

All positions are zero-based and use UTF-8 byte columns:

```json
{"line": 4, "byte_column": 8}
```

`byte_column` must be on a UTF-8 code point boundary. Kawi rejects positions inside a code point. Line endings are counted as part of the line text.

## Host methods

### editor.get_state

Request params:

```json
{"include_text": true}
```

Result:

```json
{
  "buffer": 1,
  "revision": 12,
  "path": "/home/user/project/src/main.rs",
  "language": "rust",
  "mode": "normal",
  "cursor": {"line": 4, "byte_column": 8},
  "selection": null,
  "text": "fn main() {}\n"
}
```

`include_text` defaults to `false`. `buffer` is an opaque buffer identifier and `revision` is its revision. `text` is the exact document text when requested.

### editor.apply_edit

Request params:

```json
{
  "buffer": 1,
  "revision": 12,
  "edits": [
    {
      "start": {"line": 4, "byte_column": 8},
      "end": {"line": 4, "byte_column": 12},
      "text": "value"
    }
  ]
}
```

Result:

```json
{"revision": 13}
```

Edits are applied atomically. `buffer` and `revision` must match the current buffer. A stale buffer or revision returns `-32010` and leaves the buffer unchanged. A position inside a UTF-8 code point or outside the buffer returns `-32011` and also leaves the buffer unchanged. Edits must not overlap.

### editor.execute

Request params:

```json
{"command": "file.save"}
```

Result is `null`. The command must be a built-in composable command inside the `editor.execute` allowlist. A command outside the allowlist returns `-32012`. An unknown command, a plugin-defined command, `keys.send`, or a command that requires arguments returns `-32602`. The editor runs the command synchronously and reports any execution error as `-32603`. Buffer changes use the `plugin:<name>` origin.

Aliases are accepted when the canonical command is allowed. The canonical ID is used for the allowlist and for `editor.list_commands`.

### editor.list_commands

Request params are `{}`.

Result:

```json
{
  "commands": [
    {
      "id": "file.save",
      "description": "save the buffer",
      "aliases": ["save"]
    }
  ]
}
```

The result contains only composable commands inside the allowlist, sorted by canonical ID. Plugin-defined commands and commands that require arguments are excluded.

### editor.set_diagnostics

Request params:

```json
{
  "buffer": 1,
  "revision": 12,
  "diagnostics": [
    {
      "start": {"line": 4, "byte_column": 0},
      "end": {"line": 4, "byte_column": 5},
      "severity": "warning",
      "message": "unused binding",
      "source": null,
      "code": "W1"
    }
  ]
}
```

Result is `null`.

`buffer` must be the current buffer and `revision` must match its current revision; otherwise Kawi returns `-32010`. The buffer must have a file path. Kawi replaces that plugin's diagnostics for the buffer; an empty array clears them. Severity is `error`, `warning`, `information`, or `hint`. Kawi ignores the supplied `source` and derives the displayed source from the plugin name, so a plugin cannot spoof another provider. Kawi accepts at most 1000 diagnostics per call, messages up to 4096 bytes, and codes up to 256 bytes. A payload that exceeds any of these limits returns `-32602`. Publishing is limited to 20 calls per plugin per second.

### editor.set_decorations

Request params:

```json
{
  "buffer": 1,
  "revision": 12,
  "scope": "all",
  "decorations": [
    {
      "kind": "highlight",
      "start": {"line": 4, "byte_column": 0},
      "end": {"line": 4, "byte_column": 8},
      "group": "todo"
    },
    {
      "kind": "virtual_text",
      "line": 4,
      "text": "indexed",
      "group": "todo_text"
    },
    {
      "kind": "sign",
      "line": 4,
      "text": "!",
      "group": "todo"
    }
  ]
}
```

Result is `null`.

Requires the `editor.decorations` capability. `buffer` and `revision` must match the current buffer. A stale buffer or revision returns `-32010`. `scope` defaults to `all` and accepts `all`, `visual`, or `signs`. `all` replaces every decoration published by that plugin for the buffer. `visual` replaces highlights and virtual text while preserving signs. `signs` replaces signs while preserving highlights and virtual text. An empty array clears the selected scope. A decoration kind outside the selected scope returns `-32602`. Each decoration sets exactly one of `group` or `style`. `group` must be declared in the plugin manifest. `style` is the legacy built-in fallback and accepts `accent`, `info`, `success`, `warning`, `error`, and `muted`. Highlights must be ordered and inside the buffer. A virtual text or sign line must be inside the buffer. Sign text must occupy exactly one terminal cell. Kawi accepts at most 10,000 decorations and 512 bytes of text per virtual text or sign; a payload that exceeds a limit returns `-32602`. Decorations remain until the plugin replaces them, the buffer closes, or the plugin stops. A revision change can leave them briefly stale until the next publish.

### ui.set_statusline

Request params:

```json
{
  "segments": [
    {"text": "indexed 12/12", "style": "success", "priority": 10},
    {"text": "lint", "style": "warning", "priority": 5}
  ]
}
```

Result is `null`.

Requires the `ui.statusline` capability. Kawi replaces all status line segments published by that plugin; an empty array clears them. Segments render where the `plugins` element appears in `[ui.statusline]`, ordered by descending `priority`, then by plugin name. Kawi accepts at most 16 segments and 256 bytes of text per segment. A payload that exceeds a limit returns `-32602`. Control characters and terminal escapes are removed before display.

### ui.message

Request params:

```json
{"message": "Formatted 3 files"}
```

Result is `null`. Kawi strips terminal escape sequences and replaces control characters with spaces before displaying the message.

### ui.input

Request params:

```json
{"title": "Name", "placeholder": "Ada", "value": null, "max_length": 64}
```

Result:

```json
{"kind": "text", "value": "Ada"}
```

Result is `{"kind": "canceled"}` when the user cancels. `max_length` is optional, defaults to 1024, and must be between 1 and 4096. The call suspends the invocation timeout until the user responds. Interactive calls require an integer request id and the `ui.prompt` declaration.

### ui.select

Request params:

```json
{"title": "Target", "options": ["debug", "release"], "selected": 0}
```

Result:

```json
{"kind": "selected", "value": "release"}
```

Result is `{"kind": "canceled"}` when the user cancels. Options must be non-empty, unique, and at most 512 bytes each. `selected` is the initial index and must be within bounds.

### ui.confirm

Request params:

```json
{"title": "Delete file?", "message": "This cannot be undone"}
```

Result:

```json
{"kind": "confirmed", "value": true}
```

Result is `{"kind": "canceled"}` when the user dismisses the prompt.

### ui.progress_start

Request params:

```json
{"title": "Indexing", "message": "src/main.rs", "fraction": 0.25}
```

Result:

```json
{"id": 1}
```

`fraction` is optional and must be between 0 and 1. Replaces an active progress only after the user has canceled it.

### ui.progress_update

Request params:

```json
{"id": 1, "message": "src/lib.rs", "fraction": 0.5}
```

Result:

```json
{"canceled": false}
```

`message` and `fraction` are optional. `canceled` becomes `true` after a cancel request; the plugin should stop and finish the progress.

### ui.progress_finish

Request params:

```json
{"id": 1}
```

Result is `null`. Always finish a started progress. An unknown id returns `-32602`.

### config.get

Request params:

```json
{"key": "max_items"}
```

Result:

```json
{"value": 25}
```

An unknown key returns `{"value": null}`. The value is the effective setting after applying the user's `plugins.toml` over the manifest default. `config.get` needs no capability declaration because it only exposes the calling plugin's own namespace.

### config.list

Request params are `{}` or `null`.

Result:

```json
{
  "settings": [
    {"key": "max_items", "value": 25},
    {"key": "mode", "value": "dark"}
  ]
}
```

Settings are sorted by key. Invalid user values and unknown keys are logged, and invalid values fall back to the manifest default. Kawi reads `$XDG_CONFIG_HOME/kawi/plugins.toml` on demand, so changes apply on the next call without restarting the plugin.

## Versioning

Unknown event types and unknown fields are ignored. New events and fields are additive and bump only the minor version. The plugin `events` list in the initialize result defines the intersection Kawi may send.

## Errors

| Code | Meaning |
| --- | --- |
| `-32700` | Parse error |
| `-32600` | Invalid request |
| `-32601` | Method not found or not declared |
| `-32602` | Invalid params |
| `-32603` | Internal error |
| `-32000` | Plugin command error |
| `-32010` | Stale buffer revision |
| `-32011` | Invalid position |
| `-32012` | Capability denied |
| `-32020` | Request canceled |
| `-32030` | Plugin unavailable |
| `-32040` | Interactive UI is busy |

Transport failures are host errors, not JSON-RPC errors. A process exit fails in-flight requests with `plugin process exited`.

## Restart policy

Kawi restarts a crashed process only on the next invocation. Failures use exponential backoff from 250 milliseconds up to 30 seconds. Five consecutive failures disable the plugin until `:plugin restart`. Initialization timeouts count as failures.

## Notifications

Kawi ignores unsolicited notifications except for logging. Version 1 has no host-to-plugin event stream.
