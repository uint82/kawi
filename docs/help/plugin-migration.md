---
title = "Migrating a process plugin to WASM"
aliases = ["wasm-migration"]
related = ["plugins", "plugin-authoring", "plugin-protocol", "plugin-capabilities"]
---
# Migrating a process plugin to WASM

Process plugins remain available as a trusted development runtime. Managed installs use WASM, the complete mediation path and the recommended target. This guide maps the process JSON-RPC contract to the `kawi:plugin@0.5.0` component contract. See [Writing a WASM plugin](help:plugin-authoring) for the build and release flow.

## Manifest

| Process field | WASM field |
| --- | --- |
| `runtime = "process"` or omitted | `runtime = "wasm"` |
| `command = ["python3", "${plugin_dir}/plugin.py"]` | `component = "plugin.wasm"` and `sha256 = "..."` |
| `protocol = 1` | Keep `protocol = 1`; the component targets `kawi:plugin@0.5.0` |
| `timeout_ms`, `commands`, `settings`, `subscribes`, `debounce`, `activation_events` | Unchanged |
| `capabilities` | Same names and scope shapes, now declarations rather than user grants |

A WASM manifest cannot declare `command`. A process manifest cannot declare `component` or `sha256`. Run `kawi plugin build <directory>` after every change so the component path and hash stay current.

## Lifecycle

| Process protocol | WIT contract |
| --- | --- |
| `initialize` request | `lifecycle.init(host-protocol)` |
| `invoke_command` request | `lifecycle.invoke-command(id, args)` |
| `event` notification | `lifecycle.on-event(event)` |
| `shutdown` request | No export. The host drops the instance. |
| `$/cancelRequest` | No export. The host cancels through the invocation deadline. |
| Unsolicited plugin notification | Not supported. Return values and host calls are the only output. |

`init` receives the host protocol version and runs once per instance. An error from `init` is a start failure. A trap is a crash and uses the same backoff and five-strike disable path as a process exit.

Command arguments and results use the `argument` variant. JSON numbers map to `integer` or `float`, strings to `text`, booleans to `boolean`, and null to `null`. Arrays and objects are not representable in this variant. A process plugin returns arbitrary JSON. A WASM plugin receives scalar arguments and returns one `argument`.

`invoke-command` receives only the command id and arguments. The editor context is not passed to the component. Request `editor.read` and call `editor-read.get-state` when the command needs the buffer, cursor, selection, or mode.

## Host methods

| Process method | WIT import |
| --- | --- |
| `editor.get_state` | `editor-read.get-state` |
| `editor.apply_edit` | `editor-write.apply-edit` |
| `editor.execute` | `editor-execute.execute` |
| `editor.list_commands` | `editor-execute.list-commands` |
| `config.get` | `config-read.get` |
| `config.list` | `config-read.get-all` |
| `ui.message` | `ui-message.message` |
| `ui.input`, `ui.select`, `ui.confirm` | `ui-prompt.input`, `ui-prompt.select`, `ui-prompt.confirm` |
| `ui.progress_start`, `ui.progress_update`, `ui.progress_finish` | `ui-progress.start`, `ui-progress.update`, `ui-progress.finish` |
| `editor.set_diagnostics` | `diagnostics-publish.set` |
| `editor.set_decorations` with highlights or virtual text | `editor-decorations.set` |
| `editor.set_decorations` with signs | `editor-signs.set` |
| `ui.set_statusline` | `ui-statusline.set` |
| Process execution through `process.spawn` | `process-run.run`; background jobs have no WIT equivalent in minor 10 |
| Filesystem reads and writes | WASI preopens derived from `fs.read` and `fs.write`; directories only, file grants are skipped |
| Network connections through `net.connect` | WASI sockets filtered by the network policy |

The WIT interfaces update independently. `editor-decorations.set` preserves signs, and `editor-signs.set` preserves highlights and virtual text. Process plugins get the same behavior with `scope` set to `visual` or `signs`. WIT decorations reference manifest-declared `group` strings. Process plugins may use `group` as well; `style` remains a legacy built-in fallback.

An import that is not declared by the manifest is rejected before instantiation. Kawi enables every declared API for a managed plugin, so a declared import does not need a grant. The settings methods are always available because they only expose the plugin's own namespace. Filesystem and network access use WASI instead of custom host methods, so the process JSON-RPC shapes do not carry over.

## Events

| Process event | WIT variant |
| --- | --- |
| `buffer_changed` | `buffer-changed` |
| `buffer_opened` | `buffer-opened` |
| `file_opened` | `file-opened` |
| `buffer_closed` | `buffer-closed` |
| `file_saved` | `file-saved` |
| `mode_changed` | `mode-changed` |
| `diagnostics_changed` | `diagnostics-changed` |

Field names change from snake case to kebab case. Positions and revisions keep the same meaning. A plugin receives only events present in its manifest `subscribes` list and only when the matching host API is declared.

## Host APIs

Host API names and scope shapes are the same in both runtimes. There is no grant file and no consent prompt. A managed WASM plugin receives every API declared in its manifest. Command gates still limit invocation.

| Host API | Process enforcement | WASM enforcement |
| --- | --- | --- |
| `editor.read`, `editor.write`, `editor.execute`, `editor.decorations`, `ui.message`, `ui.prompt`, `ui.progress`, `ui.statusline`, `diagnostics.read`, `diagnostics.publish`, `config.get`, `config.list` | Host mediated | Host mediated |
| `fs.read`, `fs.write` | Landlock on supported Linux kernels, otherwise advisory | WASI preopens |
| `process.spawn` | Landlock execute scope on supported Linux kernels, otherwise advisory | `process-run` host interface; background jobs are process-only in minor 10 |
| `net.connect` | Advisory | WASI socket policy with DNS pinning |

Process plugins are development-only trusted code. Managed installs require WASM.

## Errors

The WIT `error` record keeps the JSON-RPC code in `code`, the message in `message`, and the denied capability in `capability`. A process plugin returns the error in a JSON-RPC response. A WASM plugin returns `Err(error)` from a host call or lifecycle export.

| Code | Constant | Meaning |
| --- | --- | --- |
| `-32700` | `PARSE_ERROR` | Parse error. A component cannot observe framing failures. |
| `-32600` | `INVALID_REQUEST` | Invalid request. A component uses typed WIT values. |
| `-32601` | `METHOD_NOT_FOUND` | Method not found or not declared. An unlinked import cannot be called. |
| `-32602` | `INVALID_PARAMS` | Invalid params |
| `-32603` | `INTERNAL_ERROR` | Internal error |
| `-32000` | `PLUGIN_ERROR` | Plugin command error |
| `-32010` | `STALE_REVISION` | Stale buffer revision |
| `-32011` | `INVALID_POSITION` | Invalid position |
| `-32012` | `CAPABILITY_DENIED` | Capability denied |
| `-32020` | `REQUEST_CANCELED` | Request canceled |
| `-32030` | `PLUGIN_UNAVAILABLE` | Plugin unavailable |

Transport failures are host errors. A process plugin exit and a WASM trap both fail the in-flight request and count toward restart backoff.

## Positions

Both runtimes use zero-based lines and UTF-8 byte columns. A byte column inside a code point is rejected with `INVALID_POSITION`. Use `kawi_plugin_sdk::position_at` and `byte_offset` instead of counting characters.

## Build and install

1. Run `kawi plugin new <name>` to scaffold a Rust component against the published SDK. Pass `--sdk-path <path>` only for an unreleased checkout.
2. Port the lifecycle exports, host calls, and event handling.
3. Run `kawi plugin build <name>` to build the component and update `sha256`.
4. Run `kawi plugin inspect <directory>` or `:plugin inspect <name>` and compare the declared host APIs with the component imports.
5. For local development, install the directory under `$XDG_CONFIG_HOME/kawi/plugins`. For managed use, publish a GitHub release with `plugin.toml` and the component asset, then run `:plugin install owner/repo`.

Both runtimes can be installed at the same time. Keep the process plugin in development mode until the WASM plugin passes the same conformance scenarios.
