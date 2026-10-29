# WebAssembly plugin runtime

Status: phase 2 implemented behind the `wasm` feature. The runtime validates hashes, inspects imports, instantiates components, calls lifecycle functions, bridges granted host interfaces, forwards events, scopes WASI preopens, enforces epoch and memory limits, and runs authorized processes. The shared conformance harness covers core semantics, grant parity, stale revisions, queued revocation, trap and recovery, oversize components, input hygiene, WASM limits and load-time failures, and every host method and event for both runtimes. Network enforcement and the remaining host response and diagnostics payload groups remain phase 3 work. The default build does not compile or execute WebAssembly plugins.

## Principles

- Object capabilities: a plugin can use only the interfaces linked into its instance. An ungranted interface does not exist for the plugin.
- Deny by default: no preopens, no network, no process execution, and no host interfaces unless granted.
- Complete mediation: every host call crosses the same capability registry and editor request path used by process plugins.
- One contract: host methods, events, capability names, and WIT interfaces are defined once and checked for parity in tests.
- Least privilege: linking is fixed at instantiation. Revocation restarts the instance.

## Runtime

Use Wasmtime 50 with the component model and WASI Preview 2, pinned to `=50.0.0` behind the `wasm` Cargo feature. Dependency upgrades are explicit and reviewed with `cargo deny`. The initial `cargo vet` configuration uses blanket exemptions for the existing dependency tree; replace those with reviewed audits as the tree stabilizes.

Each plugin owns one `Store` on one worker thread. The editor loop never calls into WebAssembly. Host imports send a request to the editor loop over a bounded channel and wait with a deadline. The worker lazily instantiates on activation or the first invocation, calls `init` once, then serves invocations and subscribed events. A restart or shutdown cancels pending host calls before it is queued, so a blocked worker cannot delay revocation.

Limits:

- Epoch interruption with a per-call deadline.
- A two second wall-clock budget per call, checked through the epoch callback and applied to host request waits.
- Optional fuel metering when deterministic accounting is required.
- A 64 MiB memory limit, a 1,048,576 element table limit, and a 512 KiB stack limit.
- A 64 MiB component size limit.
- Bounded host message sizes, UI throttles, and diagnostic publish limits.
- One instance and one linear memory per plugin.

A trap counts as a crash and uses the existing backoff and five-strike disable path. The failed session is discarded and rebuilt on the next allowed attempt.

Epoch accounting excludes time spent blocked in host calls because the guest is not executing. Each call receives a fresh deadline. A trap marks the plugin as errored and a later invocation or restart builds a fresh instance.

## Contract

The WIT package is `kawi:plugin@0.1.0` in `crates/kawi-plugin/wit/plugin.wit`.

Interfaces:

- `editor-read` requires `editor.read`.
- `editor-write` requires `editor.write`.
- `ui-message` requires `ui.message`.
- `diagnostics-publish` requires `diagnostics.publish`.
- `process-run` requires `process.spawn`.
- `lifecycle` is exported by the plugin and contains `init`, `invoke-command`, and `on-event`.

Positions use line and UTF-8 byte column. Revision checks, origin handling, and error codes match the JSON-RPC protocol.

`fs.read`, `fs.write`, and network access use WASI instead of custom interfaces. WASI preopens are derived only from grants. The network is denied by default. A network grant is checked against recorded DNS results and rejects private and link-local addresses unless granted explicitly.

A component that imports a Kawi interface not requested by its manifest is rejected before instantiation. An import that exposes multiple capabilities requires the manifest to request all of them, while a grant of any one capability is enough to instantiate. A requested but ungranted import fails startup with the missing capability, emits a capability request to the editor, and restarts after a persistent grant. Startup prompts offer persistent grants only because linking is fixed at instantiation. Requests are deduplicated by plugin, capability, and request hash for the session, so restarts and permission reloads do not repeat a denied prompt. Multiple granted imports restart the instance once after the prompt queue drains. Linking supplies only granted interfaces. The manifest request hash includes the component hash, so replacing the binary reopens consent.

## Manifest

```toml
name = "example"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

[capabilities]
"editor.read" = true
```

`runtime` defaults to `process`. Process plugins cannot declare `component` or `sha256`. WASM plugins cannot declare `command`. The component path must stay inside the plugin directory. The declared hash must match the file at load time.

## Inspection

`:plugin inspect <name>` prints the runtime, component, computed hash, declared hash, request hash, requested capabilities, and, when the `wasm` feature is enabled, the component imports. The computed hash comes from the bytes that were hashed at load and is checked again before the component is compiled for inspection.

Import inspection runs before instantiation. It rejects imports that exceed manifest requests and reports the imports that a user is being asked to grant.

## Testing

Implemented:

- Lifecycle invocation, editor state bridging, event forwarding, denied imports, wall-clock exhaustion, blocked host-call cancellation, restart cancellation, and discarded stores after traps.
- Preopen traversal, absolute-path, symlink escape, and read-only write attempts.
- Component inspection, hash binding, malformed input, renamed imports, nested imports, and versioned WASI imports.
- Process command scope enforcement, resolution at call time, shell-free argument passing, output and runtime caps, and the capability error code.
- Shared conformance table for process and WASM runtimes: echo, denial, revocation, stale revision, grant combinations, queued revocation, trap and recovery, timeouts, invocation and event payload size, component size, input hygiene, WASM limits, WASM load-time failures, and every host method and event.
- Fixture sources under `tests/fixtures/wasm/src` rebuilt in CI with `wasm-tools`.

Still required:

- Host response and diagnostics payload size conformance rows.
- Advertised versus linked interface parity.
- Denied socket and DNS rebinding tests.
- Fuzzing of the host import boundary.

## Rollout

1. Contract, manifest fields, hash binding, component inspection, and engine foundation. This phase.
2. Lifecycle execution behind the feature flag with WASI scoping and host interfaces. Complete.
3. Network enforcement.
4. Shared conformance suite and fuzzing in CI. All groups except host response and diagnostics payload rows are complete.
5. Process runtime deprecation once the WASM authoring path is practical.

Process plugins stay supported as a labeled trusted compatibility runtime until the deprecation criteria are met.
