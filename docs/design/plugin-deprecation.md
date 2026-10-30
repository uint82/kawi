# Process plugin deprecation criteria

Status: criteria defined, no removal scheduled.

Process plugins remain supported as a labeled trusted compatibility runtime. WASM is the complete mediation path because it does not rely on the user's operating system sandbox for the host boundary.

The process runtime must not be removed until every criterion below is true and tested.

## Authoring readiness

- The WASM authoring guide and `kawi plugin new` template are shipped and exercised by CI.
- The SDK is published with the versioned WIT package, and a mismatched SDK reports a clear build or load error.
- `kawi plugin build` updates the component path and SHA-256 without manual editing.
- `kawi plugin inspect` prints requested capabilities and component imports before a plugin is installed.
- A Rust-free path is documented honestly. Other component languages are supported only when a maintained binding path exists.

## Port coverage

- A WASM echo plugin and a WASM trim plugin pass the same conformance scenarios as their process fixtures.
- The trim scenario covers `editor.get_state`, UTF-8 byte columns, `editor.apply_edit`, a stale revision retry, and a denied editor write.
- The scaffold output builds, loads, and runs its command in CI.
- The conformance matrix reports no skipped scenario for a WASM capability that has shipped.

## External validation

- At least one plugin written by someone who did not build the host is published and used for a release cycle.
- The external author reports no blocker in the authoring guide, SDK helpers, install flow, grant flow, or troubleshooting section.
- Documentation gaps found during that trial are fixed before deprecation warnings begin.

## Default and migration window

- The `wasm` feature is enabled by default for one full release cycle.
- `:plugin list` marks process plugins as a trusted compatibility runtime before any removal decision.
- Deprecation warnings run for at least two minor releases before removal.
- The migration guide maps process JSON-RPC methods, events, capabilities, and error codes to the WIT contract.
- Users can run both runtimes side by side during the entire overlap window.

## Capability decision

Process plugins stay available only if a capability cannot be expressed safely in the WASM runtime. The remaining candidate is `process.spawn`, because executing arbitrary host processes is a different trust boundary than WASI sockets and filesystem scopes.

Before removal, decide explicitly:

- whether `process.spawn` remains a process-only capability or moves to a separately reviewed WASM host process API
- whether non-Rust component languages have a supported SDK path
- whether any plugin can require the process runtime for a documented compatibility exception

If any answer keeps the process runtime alive, it stays labeled and supported. It does not stay as an undocumented fallback.
