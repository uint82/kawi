---
title = "Plugin protocol"
aliases = ["plugin-protocol-v1"]
related = ["plugins", "configuration", "keymaps"]
---
# Plugin protocol

This document defines Kawi plugin protocol version 1. The protocol is a JSON-RPC 2.0 contract between Kawi and an external process. WebAssembly plugins will implement the same contract in a later release.

## Trust model

Process plugins run with the full permissions of the user. Kawi loads plugins only from `$XDG_CONFIG_HOME/kawi/plugins`, never from a project directory. Install only plugins you trust.

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

[[commands]]
id = "ping"
title = "Ping"
description = "Return a greeting"

[commands.keybindings]
normal = ["<leader>pp"]
```

The namespace is the plugin `name`. The command above is invoked as `example.ping` and can be bound in `keymaps.toml`.

`timeout_ms` applies to initialization and each command invocation. The default is `5000`. The accepted range is `100` to `60000`.

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
- Malformed JSON, invalid framing, or oversized messages terminate the process.
- Request IDs are JSON-RPC IDs. Host requests use integer IDs.

## Lifecycle

1. Kawi reads the manifest and registers static commands and default keybindings. No process is started.
2. The first command invocation starts the process.
3. Kawi sends `initialize` and waits for the response.
4. Kawi sends `invoke_command`.
5. The process exits when Kawi shuts down or the host sends `shutdown`.

Kawi supports one in-flight invocation per plugin. Calls from the plugin to the host may happen while an invocation is running.

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
    "host_capabilities": ["editor.get_state", "editor.apply_edit", "ui.message"],
    "plugin_capabilities": ["invoke_command"]
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
    "name": "example",
    "version": "0.1.0",
    "plugin_capabilities": ["invoke_command"],
    "host_capabilities": ["editor.get_state", "editor.apply_edit", "ui.message"]
  }
}
```

Initialization fails when the protocol version differs, `invoke_command` is not declared, or the plugin declares a host capability that Kawi did not offer. Calling an undeclared host method returns `-32601`.

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
  "revision": 12,
  "path": "/home/user/project/src/main.rs",
  "language": "rust",
  "mode": "normal",
  "cursor": {"line": 4, "byte_column": 8},
  "selection": null,
  "text": "fn main() {}\n"
}
```

`include_text` defaults to `false`. `revision` is the buffer revision. `text` is the exact document text when requested.

### editor.apply_edit

Request params:

```json
{
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

Edits are applied atomically. `revision` must match the current buffer revision. A stale revision returns `-32010` and leaves the buffer unchanged. Edits must not overlap.

### ui.message

Request params:

```json
{"message": "Formatted 3 files"}
```

Result is `null`.

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
| `-32020` | Request canceled |
| `-32030` | Plugin unavailable |

Transport failures are host errors, not JSON-RPC errors. A process exit fails in-flight requests with `plugin process exited`.

## Restart policy

Kawi restarts a crashed process only on the next invocation. Failures use exponential backoff from 250 milliseconds up to 30 seconds. Five consecutive failures disable the plugin until `:plugin restart`. Initialization timeouts count as failures.

## Notifications

Kawi ignores unsolicited notifications except for logging. Version 1 has no host-to-plugin event stream.
