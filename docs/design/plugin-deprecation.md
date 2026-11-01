# Process plugin deprecation criteria

Status: authoring readiness and port coverage complete, external validation pilot ready, no independent author enrolled, default and migration window open, no removal scheduled.

Process plugins remain supported as a labeled trusted compatibility runtime. WASM is the complete mediation path because it does not rely on the user's operating system sandbox for the host boundary.

The process runtime must not be removed until every criterion below is true and tested.

## Progress

- Authoring readiness: complete. The guide, scaffold, build command, inspection command, and versioned WIT package are exercised by CI. `kawi-plugin-sdk` 0.2.0 is published by the release workflow.
- Port coverage: complete. The WASM echo and trim fixtures pass the shared conformance scenarios, including stale revision retry, retry exhaustion, buffer identity changes, and denied editor write.
- External validation: pilot ready, no independent author enrolled. The protocol, release-cycle definition, and friction form are in [plugin-external-validation](plugin-external-validation.md).
- Default and migration window: open. The `wasm` feature is enabled by default, `:plugin list` marks process plugins as a trusted compatibility runtime, and the migration guide is published. Deprecation warnings and the two-minor-release window start after external validation.
- Capability decision: complete. No capability keeps the process runtime alive.

Removal remains blocked until external validation completes and the deprecation warnings have run for at least two minor releases.

## Authoring readiness

- The WASM authoring guide and `kawi plugin new` template are shipped and exercised by CI.
- `kawi-plugin-sdk` 0.2.0 is packaged with the versioned WIT package and published by the release workflow. A mismatched SDK reports a clear build or load error. See [plugin-sdk-release](plugin-sdk-release.md).
- `kawi plugin build` updates the component path and SHA-256 without manual editing.
- `kawi plugin inspect` prints requested capabilities and component imports before a plugin is installed.
- A Rust-free path is documented honestly. Other component languages are supported only when a maintained binding path exists.

## Port coverage

- A WASM echo plugin and a WASM trim plugin pass the same conformance scenarios as their process fixtures.
- The trim scenario covers `editor.get_state`, UTF-8 byte columns, `editor.apply_edit`, a stale revision retry, retry exhaustion, a changed buffer identity, and a denied editor write.
- The scaffold output builds, loads, and runs its command in CI.
- The conformance matrix reports no skipped scenario for a WASM capability that has shipped.

## External validation

The pilot protocol, release-cycle definition, and friction form are in [plugin-external-validation](plugin-external-validation.md).

- At least one plugin written by someone who did not build the host is published and used for a release cycle.
- The external author reports no blocker in the authoring guide, SDK helpers, install flow, host API review, inspect flow, logs, or migration.
- Documentation gaps found during that trial are fixed before deprecation warnings begin.

## Default and migration window

- The `wasm` feature is enabled by default for one full release cycle.
- `:plugin list` marks process plugins as a trusted compatibility runtime before any removal decision.
- Deprecation warnings run for at least two minor releases before removal.
- The migration guide maps process JSON-RPC methods, events, capabilities, and error codes to the WIT contract.
- Users can run both runtimes side by side during the entire overlap window.

## Capability decision

Process plugins stay available only if a capability cannot be expressed safely in the WASM runtime. The remaining candidate is `process.spawn`, because executing arbitrary host processes is a different trust boundary than WASI sockets and filesystem scopes.

Decisions:

- `process.spawn` is not process-only. The WASM runtime exposes the separately reviewed `process-run` host interface and requires the same `process.spawn` declaration. It stays a host-mediated capability in both runtimes.
- Non-Rust component languages have no maintained binding path. The Rust SDK is the supported authoring path. Other component languages may target the WIT contract without support, and that does not keep the process runtime alive.
- No plugin may require the process runtime through a documented compatibility exception. Exceptions would be listed here before removal. None are approved.

No capability keeps the process runtime alive. If a future exception is approved, the process runtime stays labeled and supported. It does not stay as an undocumented fallback.
