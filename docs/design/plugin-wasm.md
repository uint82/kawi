# WebAssembly plugin runtime

Status: phases 1-4 implemented behind the `wasm` feature. The runtime validates hashes, inspects imports, instantiates components, calls lifecycle functions, bridges granted host interfaces, forwards events, scopes WASI preopens, enforces epoch and memory limits, runs authorized processes, enforces outbound TCP policy, and caps inbound and outbound bytes per connection and per session. The shared conformance harness covers core semantics, grant parity, stale revisions, queued revocation including mid-write revocation, trap and recovery, oversize invocation, event, host response, diagnostics, and component payloads, input hygiene, advertised versus linked interface parity, WASM limits and load-time failures, and every host method and event for both runtimes. Guest network rows cover explicit IP success, denied destinations, DNS lookup denial and allowlisting, UDP, bind, accept, mapped, link-local, loopback, restart socket closure, connection caps, connection reuse, inbound and outbound byte caps, session byte totals, rebinding, mixed answers, and all-private answers. Hostname-positive connect and scripted resolver rows run under the debug-only `wasm-test-network` feature; the default build still denies loopback hostnames and uses the real resolver. The default build does not compile or execute WebAssembly plugins. The separate `fuzz` workspace exercises host imports, network policy, byte wrappers, process framing, authorization, and mock process execution.

## Principles

- Object capabilities: a plugin can use only the interfaces linked into its instance. An ungranted interface does not exist for the plugin.
- Deny by default: no preopens, no network, no process execution, and no host interfaces unless granted.
- Complete mediation: every host call crosses the same capability registry and editor request path used by process plugins.
- One contract: host methods, events, capability names, and WIT interfaces are defined once and checked for parity in tests.
- Least privilege: linking is fixed at instantiation. Revocation restarts the instance.

## Runtime

Use Wasmtime 50 with the component model and WASI Preview 2, pinned exactly in the workspace manifest behind the `wasm` Cargo feature. Dependency upgrades are explicit and reviewed with `cargo deny`. The initial `cargo vet` configuration uses blanket exemptions for the existing dependency tree; replace those with reviewed audits as the tree stabilizes.

Each plugin owns one `Store` on one worker thread. The editor loop never calls into WebAssembly. Host imports send a request to the editor loop over a bounded channel and wait with a deadline. The worker lazily instantiates on activation or the first invocation, calls `init` once, then serves invocations and subscribed events. A restart or shutdown cancels pending host calls before it is queued, so a blocked worker cannot delay revocation. Revocation drops events that are still queued. An event already delivered to the guest may finish, including a host call already in flight; the conformance suite treats that in-flight write as committed and asserts it is not repeated.

Component compilation is host work and is not charged to `timeout_ms`. Compiled components are cached in memory by their verified SHA-256 hash, so manifest inspection, activation, restarts, and later sessions reuse the compiled artifact. The cache is bounded and is never populated from serialized component bytes. Instantiation and `init` are charged to the per-call budget; a timeout there is a start failure.

Limits:

- Epoch interruption with a per-call deadline.
- A wall-clock budget from the manifest `timeout_ms` per call, checked through the epoch callback and applied to host request and socket readiness waits.
- Optional fuel metering when deterministic accounting is required.
- A 64 MiB memory limit, a 1,048,576 element table limit, and a 512 KiB stack limit.
- A 64 MiB component size limit.
- An 8 MiB inbound cap and an 8 MiB outbound cap per TCP connection.
- A shared 64 MiB byte total per session across inbound and outbound streams. The session total is the binding limit for wide fan-out: 64 connections times 16 MiB per connection is 1 GiB of per-connection headroom, so a plugin hits the session total before it can use every per-connection cap. Budget work per invocation, not per connection.
- Bounded host message sizes, UI throttles, and diagnostic publish limits.
- One instance and one linear memory per plugin.

A trap counts as a crash and uses the existing backoff and five-strike disable path. The failed session is discarded and rebuilt on the next allowed attempt.

Epoch accounting excludes time spent blocked in host calls because the guest is not executing. Each call receives a fresh deadline. A trap marks the plugin as errored and a later invocation or restart builds a fresh instance.

## Contract

The WIT package is `kawi:plugin@0.1.0` in `crates/kawi-plugin-sdk/wit/plugin.wit`. The SDK owns the contract and the runtime consumes the same file.

Interfaces:

- `editor-read` requires `editor.read`.
- `editor-write` requires `editor.write`.
- `ui-message` requires `ui.message`.
- `diagnostics-publish` requires `diagnostics.publish`.
- `process-run` requires `process.spawn`.
- `lifecycle` is exported by the plugin and contains `init`, `invoke-command`, and `on-event`.

Positions use line and UTF-8 byte column. Revision checks, origin handling, and error codes match the JSON-RPC protocol.

`fs.read`, `fs.write`, and network access use WASI instead of custom interfaces. WASI preopens are derived only from grants. The network is denied by default.

## Network

`net.connect` grants enable outbound WASI TCP connect only. UDP, listen, accept, and bind stay disabled except for the wildcard ephemeral bind that connect needs. Hostname grants are resolved once when the session is built with a five-second cap, and only globally routable answers are pinned. Mixed answers pin only the public addresses. The session never re-resolves a hostname, so TTL expiry cannot widen the pinned set; a long-lived session keeps the addresses pinned at build time. Explicit IP grants allow that exact address and port, including loopback and private addresses. The socket address check permits TCP connect only to a pinned address and permits the wildcard ephemeral bind that connect needs. A hostname that resolves to a private, loopback, link-local, shared, documentation, benchmarking, multicast, reserved, 6to4, Teredo, or IPv4-mapped non-public address is denied.

WASI name lookup is replaced with a filtered resolver. The guest can resolve only hostnames present in the grant, and the resolver returns only the public addresses pinned for that hostname. IP literals are returned without an external request when name lookup is enabled. Resolution failure fails closed with no pinned addresses. A failed or timed-out resolution emits a `network warning` that appears in `:plugin list` and `:plugin logs`; the plugin still starts, and the warning does not count toward the five-failure disable. Arbitrary names are denied before any DNS request. The conformance suite exercises denied lookup, non-granted lookup, IP literal lookup, hostname-positive connect, rebinding, mixed answers, and all-private answers through the guest. The debug-only test resolver is scripted per session and counts calls, so the suite asserts one resolution per live session, no pin-set widening from later answers, and re-resolution after restart. An all-private answer produces a clean name lookup error rather than an empty address list, and never a trap.

Each session allows at most 64 simultaneous TCP connections. A connection releases its slot when the guest drops the socket or shuts down both directions, so connect-and-disconnect loops do not exhaust the cap. The cap is per plugin session, not global. Denied attempts do not consume a slot. Each connection also has an 8 MiB inbound cap and an 8 MiB outbound cap. Reads are limited to the remaining budget, so a read that crosses the cap returns a short read; the next read returns EOF. Writes are limited by the permit, so a write that crosses the cap is shortened to the remaining budget; the next write fails as closed, and the socket stays open for reads. Socket readiness waits use the invocation budget in both directions, so a peer that stops reading or sending cannot hold the call past that budget. A handled cap hit is a normal stream error and does not count as a crash; an unhandled one follows the normal invocation failure path. The conformance suite checks the plugin is still ready immediately after each cap and then makes a successful call. Reconnecting starts fresh per-connection budgets. All streams in a session share a 64 MiB byte total across inbound and outbound. When the session total is exhausted, reads return EOF and writes fail as closed. A restart, or a grant change that restarts the plugin, builds a new session, resets the budgets, and re-resolves hostnames. A live session never re-resolves, so a service that changes IP address keeps the old pinned address until restart. Resolution happens while the session is built, not inside an invocation, so the five-second cap is a bounded startup cost rather than part of the invocation budget. A timeout fails closed and leaves the worker restartable.

Known limits: the caps apply to WASI TCP streams, not process plugins. The session total is shared across both directions, so a plugin that reads a large amount has less room to write. The caps count bytes at the WASI stream boundary, not kernel buffers.

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
- Shared conformance table for process and WASM runtimes: echo, denial, revocation, stale revision, grant combinations, queued revocation, mid-write revocation, trap and recovery, timeouts, invocation, event, host response, diagnostics, and component payload size, advertised versus linked interface parity, input hygiene, WASM limits, WASM load-time failures, and every host method and event.
- Fixture sources under `tests/fixtures/wasm/src` rebuilt in CI with `wasm-tools`.
- Guest network fixtures under `tests/fixtures/wasm/network`, rebuilt in CI with the `wasm32-wasip2` target and `wasm-component-ld`.
- Network policy tests for DNS pinning, rebinding, mixed answers, resolver failure, resolver timeout, connection caps, filtered name lookup, mapped addresses, and address classification.
- Guest restart-closure, connection-cap, connection-reuse, and byte cap rows run in the default WASM build. The cap rows cover exact inbound and outbound limits, straddling short reads and writes, a vectored write path, a splice path, a peer that stops reading, a peer that stops sending, mixed-direction session accounting, repeated close and reconnect, and the shared session total. Hostname-positive, rebinding, mixed-answer, and all-private rows use the debug-only `wasm-test-network` feature. Without that feature the scripted rows are reported as not-supported under `TestResolverRequired` instead of running without a resolver.
- Release builds for the default and `wasm` feature sets, plus a feature-graph check that fails if `wasm-test-network` is enabled.
- Fuzz targets for network address parsing, pinned-set policy, byte-counting wrappers, JSON-RPC framing, host-method authorization, editor and diagnostics semantics, sanitization, error normalization, and mock process authorization.

## Fuzzing

The `fuzz` workspace is excluded from the main workspace and runs on nightly with `cargo-fuzz` and `libfuzzer-sys`. Targets call host-side functions directly and use fake streams, a fake resolver, and a mock process executor. They assert capability denials produce no editor effect, edits preserve UTF-8 and revisions, diagnostics respect the 1000/4096/256 caps, sanitized text has no control characters, errors use the conformance normalization set, and ungranted commands are denied without spawning.

Fuzzing is complete. Run targets with `fuzz/run.sh smoke` for the 30 second PR check or `fuzz/run.sh nightly <target>` for the scheduled budget. The runner sets `-max_len`, `-rss_limit_mb`, and `-malloc_limit_mb` per mode. Pull requests that touch the host boundary run the smoke job. The weekly schedule and manual dispatch run the six targets in a parallel matrix at 120 seconds per target, cache the corpus, and upload crash artifacts. See `fuzz/README.md` for the deliberate `libfuzzer-sys` vet exclusion and the line and region coverage model.

## Rollout

1. Contract, manifest fields, hash binding, component inspection, and engine foundation. Complete.
2. Lifecycle execution behind the feature flag with WASI scoping and host interfaces. Complete.
3. Network enforcement. Complete.
4. Shared conformance suite and fuzzing in CI. Complete.
5. Process runtime deprecation once the WASM authoring path is practical. Criteria defined in [plugin-deprecation](plugin-deprecation.md).

Process plugins stay supported as a labeled trusted compatibility runtime until the deprecation criteria are met.
