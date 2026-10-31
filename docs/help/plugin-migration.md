---
title = "Migrating a process plugin to WASM"
aliases = ["wasm-migration"]
related = ["plugins", "plugin-authoring", "plugin-protocol", "plugin-capabilities"]
---
# Migrating a process plugin to WASM

Process plugins remain available as a trusted compatibility runtime. WASM is the complete mediation path and the recommended target. This guide maps the process JSON-RPC contract to the `kawi:plugin@0.1.0` component contract. See [Writing a WASM plugin](help:plugin-authoring) for the build and install flow.

## Manifest

| Process field | WASM field |
| --- | --- |
| `runtime = "process"` or omitted | `runtime = "wasm"` |
| `command = ["python3", "${plugin_dir}/plugin.py"]` | `component = "plugin.wasm"` and `sha256 = "..."` |
| `protocol = 1` | Keep `protocol = 1`; the component targets `kawi:plugin@0.1.0` |
| `timeout_ms`, `commands`, `subscribes`, `debounce`, `activation_events` | Unchanged |
| `capabilities` | Unchanged names and scope shapes |

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
| `ui.message` | `ui-message.message` |
| `editor.set_diagnostics` | `diagnostics-publish.set` |
| Process execution through `process.spawn` | `process-run.run` |
| Filesystem reads and writes | WASI preopens derived from `fs.read` and `fs.write`; directories only, file grants are skipped |
| Network connections through `net.connect` | WASI sockets filtered by the network policy |

An import that is not requested by the manifest is rejected before instantiation. An import that is requested but not granted fails startup with the missing capability. Filesystem and network access use WASI instead of custom host methods, so the process JSON-RPC shapes do not carry over.

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

Field names change from snake case to kebab case. Positions and revisions keep the same meaning. A plugin receives only events present in its manifest `subscribes` list and only when the matching capability is granted.

## Capabilities

Capability names and scope shapes are the same in both runtimes. The default tier, grant file, approval diff, command gates, and consent prompt apply to both.

| Capability | Process enforcement | WASM enforcement |
| --- | --- | --- |
| `editor.read`, `editor.write`, `ui.message`, `diagnostics.read`, `diagnostics.publish` | Host mediated | Host mediated |
| `fs.read`, `fs.write` | Landlock on supported Linux kernels, otherwise advisory | WASI preopens |
| `process.spawn` | Landlock execute scope on supported Linux kernels, otherwise advisory | `process-run` host interface |
| `net.connect` | Advisory | WASI socket policy with DNS pinning |

WASM startup prompts offer persistent grants only because linking is fixed at instantiation. Process prompts can still offer allow once.

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
4. Run `kawi plugin inspect <directory>` or `:plugin inspect <name>` and compare the requested capabilities with the component imports.
5. Install the directory under `$XDG_CONFIG_HOME/kawi/plugins` and grant the requested capabilities.

Both runtimes can be installed at the same time. Keep the process plugin until the WASM plugin passes the same conformance scenarios and the grant flow is verified.
