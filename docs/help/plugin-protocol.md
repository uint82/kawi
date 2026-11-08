---
title = "Plugin protocol"
aliases = ["plugin-protocol-v1"]
related = ["plugins", "plugin-capabilities", "plugin-authoring", "plugin-migration", "configuration", "keymaps"]
---
# Plugin protocol

This document defines Kawi plugin protocol version 1. Minor version 1 adds events. Minor version 2 adds plugin-published diagnostics. Minor version 3 adds editor command composition. Minor version 4 adds plugin settings. Minor version 7 adds buffer decorations and status line segments. Minor version 8 adds decoration scopes. Minor version 9 adds named decoration groups. Minor version 10 adds background process jobs for process plugins. Minor version 11 adds WASM job parity and `process.poll`. Minor version 12 adds plugin panels. A plugin that implements an earlier minor version still initializes and receives its declared events; it simply does not use the newer host methods. The protocol is a JSON-RPC 2.0 contract between Kawi and an external process. WebAssembly plugins implement the same behavior through the versioned WIT contract described in [Migrating a process plugin to WASM](help:plugin-migration). Process plugins receive job events as notifications; WASM plugins drain queued job events with `process.poll`.

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
    "protocol_minor": 10,
    "host_capabilities": ["editor.get_state", "editor.apply_edit", "editor.execute", "editor.list_commands", "editor.set_diagnostics", "editor.set_decorations", "ui.set_statusline", "ui.message", "ui.input", "ui.select", "ui.confirm", "ui.progress_start", "ui.progress_update", "ui.progress_finish", "ui.panel_open", "ui.panel_update", "ui.panel_focus", "ui.panel_close", "config.get", "config.list"],
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
    "protocol_minor": 10,
    "name": "example",
    "version": "0.1.0",
    "plugin_capabilities": ["invoke_command"],
    "host_capabilities": ["editor.get_state", "editor.apply_edit", "editor.execute", "editor.list_commands", "editor.set_diagnostics", "editor.set_decorations", "ui.set_statusline", "ui.message", "ui.input", "ui.select", "ui.confirm", "ui.progress_start", "ui.progress_update", "ui.progress_finish", "ui.panel_open", "ui.panel_update", "ui.panel_focus", "ui.panel_close", "config.get", "config.list"],
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

### ui.panel_open

Request params:

```json
{
  "title": "Index",
  "status": "building",
  "height": 8,
  "focus": false,
  "lines": [
    {"text": "src/main.rs", "style": "success"},
    {"text": "src/lib.rs"}
  ]
}
```

Result:

```json
{"panel": 1, "width": 80, "height": 8}
```

Requires the `ui.panel` capability. Only one plugin panel can be active across all plugins. A second open returns `-32040`. `status` and `height` are optional. `height` is clamped to 4 through 18 and defaults to 8. `focus` requests keyboard focus without stealing it when the panel cannot fit. The result reports the actual width and height, which can be 0 when the editor has no space.

Kawi accepts at most 1000 lines, 4096 bytes per line, 1 MiB of total line text, a 256-byte title, and a 256-byte status. A payload that exceeds a limit returns `-32602`. Control characters and terminal escapes are removed before display. Each line may set a `style` of `accent`, `info`, `success`, `warning`, `error`, or `muted`.

### ui.panel_update

Request params:

```json
{
  "panel": 1,
  "status": "complete",
  "height": 6,
  "lines": [
    {"text": "indexed 42 files", "style": "success"}
  ]
}
```

Result is `null`. Every field except `panel` is optional. Omitted fields keep their current value, and omitted `lines` preserves the current content. The same limits as `ui.panel_open` apply. An unknown panel or a panel owned by another plugin returns `-32060`.

### ui.panel_focus

Request params:

```json
{"panel": 1}
```

Result is `null`. Focus moves from the editor, terminal, debug panel, or explorer to the plugin panel. An unknown panel or a panel owned by another plugin returns `-32060`.

### ui.panel_close

Request params:

```json
{"panel": 1}
```

Result is `null`. Closing removes the panel and returns focus to the editor. An unknown panel or a panel owned by another plugin returns `-32060`.

The host owns panel layout, focus, scrolling, and closing. The panel is docked above the task panel and reserves at least four editor rows. It hides if the editor becomes too small and is removed when its plugin stops. While focused, `j`, `k`, the arrow keys, the mouse wheel, `Ctrl+d`, `Ctrl+u`, `PageDown`, `PageUp`, `g`, `G`, `Home`, and `End` scroll; `Esc` returns focus to the editor; `q` closes the panel and `x` is also accepted. The `plugin.panel` command toggles focus, and `:plugin panel close` closes it. There is no raw key input API in protocol minor 12.

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

## Process jobs

Background jobs let a running process plugin start a long-lived child, stream its output, write to its stdin, signal it, and observe its exit. A job keeps running after the command invocation that started it returns. Output and exit events are delivered to the running plugin while no invocation is in flight.

Jobs require the `process.spawn` capability and use the same command scope as the WebAssembly `process-run` interface. Protocol minor 10 exposes jobs to process plugins, and minor 11 exposes them to WASM plugins through the `process-jobs` WIT interface. Process plugins receive `job_output` and `job_exited` notifications while idle. WASM plugins are passive, so Kawi queues the same events and `process.poll` drains them on the next host call.

### process.open

Request params:

```json
{
  "command": "/usr/bin/gdb",
  "args": ["--interpreter=dap"],
  "cwd": "$WORKSPACE",
  "env": {"RUST_LOG": "debug"},
  "stdin": "pipe"
}
```

Result:

```json
{"job": 1}
```

`command` must resolve to an executable inside the `process.spawn` command scope. Arguments are not scoped. `cwd` defaults to the plugin directory. A relative `cwd` resolves from the plugin directory. An absolute `cwd` must resolve inside the plugin directory, the workspace, or the configuration directory. `env` adds or overrides entries in the inherited environment. `stdin` is `pipe` by default; `null` connects the child to an empty stdin.

### job_output

Kawi sends `job_output` notifications while the plugin is idle:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "event": "job_output",
    "payload": {
      "job": 1,
      "stream": "stdout",
      "data": "Content-Length: 42\r\n\r\n",
      "truncated": false
    }
  }
}
```

`stream` is `stdout` or `stderr`. `data` is UTF-8 text decoded incrementally; invalid byte sequences are replaced with U+FFFD. `truncated` is `true` on a single event after the job reaches the output limit. Kawi continues to drain the child streams after the limit but discards the excess, so a job cannot block on a full pipe.

### job_exited

Kawi sends `job_exited` after the child exits and the remaining output is queued:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "event": "job_exited",
    "payload": {
      "job": 1,
      "state": "exited",
      "exit_code": 0,
      "signal": null
    }
  }
}
```

`state` is `exited`, `killed`, or `failed`. `exit_code` is `null` when the process was terminated by a signal or when the status is unavailable. `signal` is the terminating signal number on Unix. Kawi waits up to two seconds for stdout and stderr to drain before sending `job_exited`, so final output normally arrives first.

Job events are tied to the job that produced them and are not listed in `subscribes`. A plugin receives them while the job exists, including while no invocation is in flight.

### process.write

Request params:

```json
{"job": 1, "data": "continue\n"}
```

Result is `null`. `data` is UTF-8 text and is limited to 1 MiB per call. The write is queued without waiting for the child to read it. A full queue returns `-32053`; a closed or exited job returns `-32052`. Writing to a job opened with `"stdin": "null"` returns `-32052`.

### process.close_stdin

Request params:

```json
{"job": 1}
```

Result is `null`. Closing is idempotent, including after the job exits. After the call, the child sees end of file on stdin and later writes return `-32052`.

### process.signal

Request params:

```json
{"job": 1, "signal": "terminate"}
```

Result is `null`. `signal` is `interrupt`, `terminate`, or `kill`. On Unix, Kawi sends the signal to the child process group. `kill` marks the job `killed` after delivery; `interrupt` and `terminate` leave the state `running` until the process exits. A signal to an exited job returns `-32052`. On other platforms only `kill` is supported; the other values return `-32030`.

### process.kill

Request params:

```json
{"job": 1}
```

Result is `null`. Kawi sends a hard kill to the process group, waits for the child, and marks the job `killed`. Killing an already exited job is a no-op.

### process.status

Request params:

```json
{"job": 1}
```

Result:

```json
{"state": "running", "exit_code": null, "signal": null}
```

Use `process.status` for polling and `job_exited` for waiting. There is no separate wait method or started event; `process.open` returns the job ID synchronously and every event carries it. WASM plugins use `process.status` and `process.poll`.

### process.poll

Request params:

```json
{"job": 1}
```

Result is a list of queued job events:

```json
[
  {"event": "job_output", "payload": {"job": 1, "stream": "stdout", "data": "hello", "truncated": false}},
  {"event": "job_exited", "payload": {"job": 1, "state": "exited", "exit_code": 0, "signal": null}}
]
```

`process.poll` drains the queue for one job and returns oldest events first. It is intended for WASM plugins, which are passive and cannot receive notifications while idle. Process plugins normally use `job_output` and `job_exited` notifications; polling a process job returns the events that were queued for WASM delivery, which is empty for the process runtime. The queue holds up to 1024 events or 8 MiB of output per job, whichever is reached first, and drops the oldest events when full. The exit event remains available through `process.status` even if the queue drops it.

### Limits

- A plugin may have 16 running jobs. Starting another one returns `-32051`.
- A job may emit 8 MiB across stdout and stderr. Kawi truncates the stream after that limit.
- A job may receive 1 MiB of stdin per `process.write` call.
- A job may pass 256 arguments and set 64 environment entries. Arguments are limited to 64 KiB each and 1 MiB in total; environment keys, values, and total size are limited to 1 KiB, 32 KiB, and 128 KiB.
- Kawi retains exited jobs for five minutes so late `status` calls succeed.
- Process job events use the per-plugin notification queue. If that queue is full, Kawi drops the event; the job keeps running and remains observable through `process.status`.
- WASM job events use a per-job queue capped at 1024 events or 8 MiB. `process.poll` drains the oldest events first; a full queue drops the oldest output.

Stopping, crashing, restarting, reloading, or disabling a plugin kills every job owned by that plugin.

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
| `-32050` | Job not found |
| `-32051` | Too many running jobs |
| `-32052` | Job or job stdin is closed |
| `-32053` | Job output or input queue is full |
| `-32060` | Panel not found or owned by another plugin |

Transport failures are host errors, not JSON-RPC errors. A process exit fails in-flight requests with `plugin process exited`.

## Restart policy

Kawi restarts a crashed process only on the next invocation. Failures use exponential backoff from 250 milliseconds up to 30 seconds. Five consecutive failures disable the plugin until `:plugin restart`. Initialization timeouts count as failures.

## Notifications

Kawi ignores unsolicited notifications except for logging. Kawi sends editor events and job events as `event` notifications. Editor events respect `subscribes`; job events are delivered for jobs the plugin owns.
