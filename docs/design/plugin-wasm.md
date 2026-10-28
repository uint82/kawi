# WebAssembly plugin runtime

Status: proposed. Phase 1 adds the contract, manifest fields, component inspection, and the feature-gated Wasmtime foundation. It does not execute WebAssembly plugins yet.

## Principles

- Object capabilities: a plugin can use only the interfaces linked into its instance. An ungranted interface does not exist for the plugin.
- Deny by default: no preopens, no network, no process execution, and no host interfaces unless granted.
- Complete mediation: every host call crosses the same capability registry and editor request path used by process plugins.
- One contract: host methods, events, capability names, and WIT interfaces are defined once and checked for parity in tests.
- Least privilege: linking is fixed at instantiation. Revocation restarts the instance.

## Runtime

Use Wasmtime 50 with the component model and WASI Preview 2, pinned to `=50.0.0` behind the `wasm` Cargo feature. Dependency upgrades are explicit and reviewed with `cargo deny`. `cargo vet` audits are a follow-up once the initial dependency set is stable.

Each plugin owns one `Store` on one worker thread. The editor loop never calls into WebAssembly. Host imports send a request to the editor loop over a bounded channel and wait with a deadline.

Limits:

- Epoch interruption with a per-call deadline.
- Optional fuel metering when deterministic accounting is required.
- Hard memory, table, instance, and stack limits.
- Bounded host message sizes, UI throttles, and diagnostic publish limits.
- One instance and one linear memory per plugin.

A trap counts as a crash and uses the existing backoff and five-strike disable path. Queued events are cleared and buffers are resynchronized.

Epoch accounting includes time spent in host calls. A host call must therefore consume a bounded part of the deadline or pause the budget explicitly.

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

A component that imports a Kawi interface not requested by its manifest is rejected before instantiation. Linking supplies only granted interfaces. The manifest request hash includes the component hash, so replacing the binary reopens consent.

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

`kawi plugin inspect <name>` prints the runtime, component, declared hash, request hash, requested capabilities, and, when the `wasm` feature is enabled, the component imports.

Import inspection runs before instantiation. It rejects imports that exceed manifest requests and reports the imports that a user is being asked to grant.

## Testing

Required before execution is enabled:

- Conformance table shared by process and WASM runtimes: echo, trim, events, denial, revocation, and stale revision.
- Filesystem escape and symlink escape through a preopen.
- Denied socket, DNS rebinding, epoch exhaustion, memory trap, oversize payload, and trap mid-invocation.
- Import versus manifest parity and advertised versus linked interface parity.
- Fuzzing of the host import boundary.

## Rollout

1. Contract, manifest fields, hash binding, component inspection, and engine foundation. This phase.
2. Lifecycle execution behind the feature flag with WASI scoping and host interfaces.
3. Network and process enforcement.
4. Shared conformance suite and fuzzing in CI.
5. Process runtime deprecation once the WASM authoring path is practical.

Process plugins stay supported as a labeled trusted compatibility runtime until the deprecation criteria are met.
