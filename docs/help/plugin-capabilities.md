---
title = "Plugin capabilities"
aliases = ["permissions"]
related = ["plugins", "plugin-protocol", "configuration", "troubleshooting"]
---
# Plugin capabilities

Capabilities define what a plugin may ask Kawi to do. The plugin requests capabilities in its manifest. The user grants capabilities in a separate file that the plugin does not control. Kawi enforces the intersection and denies everything else.

Kawi enforces the `editor.*` and `ui.*` host methods, `editor.set_diagnostics`, and `diagnostics.read` events for process plugins. On Linux with a full Landlock ABI, Landlock enforces `fs.read`, `fs.write`, and `process.spawn` for process plugins. The WebAssembly runtime enforces `net.connect` with a deny-by-default WASI socket policy. Landlock cannot scope network destinations, so `net.connect` remains advisory for process plugins. On other platforms every filesystem, process, and network scope remains advisory for process plugins.

## Capability list

| Capability | Scope | Notes |
| --- | --- | --- |
| `editor.read` | none | Buffer text, metadata, cursor, selection, mode, revisions, and editor events |
| `editor.write` | none | `editor.apply_edit` and future buffer mutations |
| `ui.message` | none | Status messages. Default tier |
| `diagnostics.read` | none | Host diagnostic events |
| `diagnostics.publish` | none | `editor.set_diagnostics` for the current buffer |
| `fs.read` | paths | Read files under the granted paths |
| `fs.write` | paths | Write files under the granted paths |
| `process.spawn` | commands | Spawn the granted executables |
| `net.connect` | hosts | Open outbound TCP connections to the granted hosts |

Names are stable. Unknown capabilities are ignored with a warning. New capabilities are additive and bump only the protocol minor version.

## Scopes

Unscoped capabilities use `true` in both requests and grants. Scoped capabilities use tables.

### Paths

`fs.read` and `fs.write` take `paths`, an array of directories or files. An import that exposes multiple capabilities, such as WASI filesystem read and write, must have every exposed capability requested by the manifest. A grant for any one of the requested capabilities is enough to instantiate the plugin, so read-only access still grants only `fs.read`.

Paths may start with these variables:

- `$WORKSPACE`: the active workspace roots
- `$PLUGIN_DIR`: the plugin directory
- `$CONFIG`: `$XDG_CONFIG_HOME/kawi`
- `$HOME`: the user home directory

Other variables are rejected. Kawi expands variables when it builds the Landlock ruleset at spawn time. `$WORKSPACE` is the active project root, which falls back to the current directory when no project is open, captured for the life of the plugin process.

Expanded paths must be absolute. Kawi rejects `..` components before resolving. Grant paths are canonicalized when the sandbox is built and symlinks are resolved before the rule is installed, so a symlink that escapes a grant does not match. A grant path that does not exist is skipped until the path exists and the plugin restarts.

Path scope is a component prefix. `/home/u/proj/src` is narrower than `/home/u/proj`. `/home/u/projection` is not inside `/home/u/proj`. A trailing separator is ignored. A grant for a file covers exactly that file. A grant never widens a request.

### Commands

`process.spawn` takes `commands`, an array of executable names or absolute paths. Kawi resolves bare names with the host PATH at the time of the call. Request and grant entries are both resolved and canonicalized to absolute executable paths. A grant covers only the same canonical path. Matching is never by name alone. A bare grant for `git` does not cover a request for an absolute path that resolves elsewhere. An executable that cannot be resolved is denied. Arguments are not scoped.

### Hosts

`net.connect` takes `hosts`, an array of exact `host:port` entries. IPv6 uses brackets. Kawi lowercases hostnames before comparison. Matching is exact. `api.example.com:443` is not covered by a grant for `*.example.com:443`, and wildcards are invalid in version 1. Port ranges are not supported.

The WebAssembly runtime resolves hostname grants once per session and pins the resulting addresses. Only globally routable answers are pinned. Loopback, private, link-local, shared, documentation, benchmarking, multicast, and reserved answers are rejected unless the grant names the address literally. A DNS name that rebinds to a different or non-public address is denied for the life of the session. IP literal grants allow that exact address and port, including loopback and private addresses. The guest may resolve names only when a hostname grant exists. TCP connect is allowed only to a pinned address. UDP, listen, and accept are denied.

### Subsumption summary

| Scope | A grant covers a request when |
| --- | --- |
| none | both are unscoped |
| paths | the granted canonical path is the requested path or a descendant component prefix |
| commands | both resolve to the same canonical executable path |
| hosts | host and port match exactly |

## Manifest request

Plugins request capabilities in `plugin.toml`:

```toml
[capabilities]
"editor.read" = true
"editor.write" = true
"ui.message" = true
"fs.read" = { paths = ["$WORKSPACE"] }
"process.spawn" = { commands = ["git"] }
"net.connect" = { hosts = ["api.example.com:443"] }
```

Rules:

- Unscoped capabilities must be `true`.
- Scoped capabilities must be a table with the documented key.
- `true` for a scoped capability is invalid and produces a warning.
- Unknown capability names and unknown scope keys produce warnings and are ignored.
- Registering commands and default keybindings needs no capability. They only publish names.

The manifest is a request. It is not consent.

## Grant file

Users grant capabilities in `$XDG_CONFIG_HOME/kawi/permissions.toml`:

```toml
[plugins.trim.grant]
"editor.read" = true
"editor.write" = true
"ui.message" = true

[plugins.gitsigns.grant]
"editor.read" = true
"process.spawn" = { commands = ["git"] }

[plugins.example]
request_hash = "sha256:6f0f2b6e..."

[plugins.example.grant]
"ui.message" = false
```

`grant` is a table with the same string-or-table shape as the manifest request. `false` revokes a default-tier capability. This keeps the format valid TOML and avoids heterogeneous arrays.

Rules:

- The grant file lives in user configuration. Kawi never reads a grant file from a plugin directory and warns when one is present there.
- Every entry in the `grant` table must be a request in the manifest. Unrequested grants add no access and are reported as stale by doctor.
- Grants for unknown plugins add no access and are reported as stale by doctor.
- A scoped grant must be equal to or narrower than the request scope.
- Multiple grants for one capability are combined.
- `false` is valid only for default-tier capabilities. It revokes that default. Using it for any other capability warns and is ignored.
- A missing grant file grants only the default tier.
- A malformed grant file fails closed: every plugin receives only the default tier, with a startup error and doctor output. Kawi does not grant everything and does not abort startup.
- An invalid scope in one entry drops that entry only. Other valid grants in the file still apply.
- Kawi warns when the grant file is writable by group or other users.

Kawi watches the parent directory of `permissions.toml` and reloads it after a debounced change. Reloads use the same atomic fail-closed swap as `:plugin reload`. A malformed save drops every plugin to the default tier and warns instead of keeping the previous grants. Kawi suppresses the reload triggered by its own grant writes. Revocation clears any one-shot allowances. Host-method grants apply to the next call; a change to an `fs.*` or `process.spawn` scope restarts the affected plugin so the kernel ruleset matches the new grants.

`request_hash` is optional. When present, it is the SHA-256 of the canonical request set. The hash input is compact UTF-8 JSON with no whitespace:

```json
{"capabilities":{"editor.read":true,"fs.read":{"paths":["$WORKSPACE/src"]}}}
```

Canonicalization applies these rules:

- Capability names and scope keys are sorted lexicographically.
- Scope arrays are deduplicated and sorted after lexical normalization.
- Paths reject `..` components and normalize `.` and duplicate separators. Hashing does not expand variables, resolve the PATH, or resolve symlinks.
- Hostnames are lowercased. Command strings keep their case.
- Unknown or invalid entries are omitted from the hash and reported separately.

Reordering a manifest does not change the hash. Widening a scope does. A mismatch warns that the plugin request changed and withholds all grants except the default tier until the user updates the grant file. This is conservative because the permissions file stores only one hash, so Kawi cannot identify which individual request changed. Adding, widening, or shrinking a request re-flags the plugin as changed.

## Effective grant

The effective grant for a plugin is:

1. The default tier, currently `ui.message`, unless the grant table sets `"ui.message" = false`.
2. The explicit `grant` table for that plugin.
3. Intersected with the manifest request set.

Only default-tier capabilities may be revoked with `false`. A grant for a capability the plugin did not request adds no access. A plugin update that adds a request never gains access because the new request is outside the existing grant. The plugin starts with the intersection and warns about the rest. Kawi stores no mutable grant state inside the plugin directory.

Scope narrowing examples:

- Request `fs.read = { paths = ["$WORKSPACE"] }`, grant `"fs.read" = { paths = ["$WORKSPACE/src"] }`: covers `src` only.
- Request `paths = ["$WORKSPACE/src"]`, grant `paths = ["$WORKSPACE"]`: does not cover the request.
- Request `process.spawn = { commands = ["git", "rg"] }`, grant `"process.spawn" = { commands = ["git"] }`: grants `git` only and warns about `rg`.
- Request `net.connect = { hosts = ["api.example.com:443"] }`, grant `"net.connect" = { hosts = ["api.example.com:443"] }`: exact match.

## Enforcement table

| Method or event | Capability |
| --- | --- |
| `initialize` | none |
| `invoke_command` | none |
| `shutdown` | none |
| `$/cancelRequest` | none |
| `on_startup`, `on_command`, `on_file_open` | none |
| `editor.get_state` | `editor.read` |
| `editor.apply_edit` | `editor.write` |
| `ui.message` | `ui.message` |
| `editor.set_diagnostics` | `diagnostics.publish` |
| `buffer_changed` | `editor.read` |
| `buffer_opened` | `editor.read` |
| `file_opened` | `editor.read` |
| `buffer_closed` | `editor.read` |
| `file_saved` | `editor.read` |
| `mode_changed` | `editor.read` |
| `diagnostics_changed` | `diagnostics.read` |

Kawi authorizes a host method before reading its parameters, looking up a buffer, or comparing revisions. A plugin without `editor.read` cannot use `-32010` stale-revision errors to probe buffer existence or revision numbers. Unknown methods return `-32601`; known methods without an effective grant return `CAPABILITY_DENIED`.

Host requests are checked against the current grants on every call. A JSON-RPC notification has no id, so a denied notification is dropped and recorded in `:plugin logs` instead of receiving an error response.

Events are checked when they enter the queue and again when the writer writes them to the plugin pipe. Revoking a grant therefore drops events already queued for a stalled plugin. A plugin without `editor.read` receives no editor events, even if it subscribes to them. A plugin without `diagnostics.read` receives no diagnostic events. Startup and doctor warn when a subscription is unusable because of a missing capability. Event payloads that carry paths, including `buffer_opened`, `file_opened`, and `file_saved`, use the same gate.

The `host_capabilities` and `events` fields in `initialize` are derived from the same table as enforcement. Kawi advertises only methods and events covered by the effective grant at initialization time. Enforcement continues to use the current grants, so host-method revocation applies to the next message without restarting the plugin. Filesystem and execute scopes are fixed for the life of the process and change on restart.

The `invoke_command` context is editor state. Without `editor.read`, Kawi replaces it with an empty context: buffer and revision `0`, no path or language, normal mode, cursor at the origin, and no selection. Command arguments are caller-supplied command input from configuration or keybindings, not editor state, and are passed through.

## Consent prompt

A denial opens a host prompt with three choices: allow once, allow always, or deny. The prompt is the normal consent path. `:plugin grant <plugin> <capability>` opens the same prompt for the typed flow. WASM startup imports offer only allow always and deny because linking is fixed at instantiation.

- Only real keyboard input answers a prompt. Synthetic keys from `keys.send`, macro replay, mouse events, and pasted text are ignored while a prompt is open.
- Plugin-originated messages cannot replace the prompt text.
- Plugin names, changed-capability summaries, and other rendered values are stripped of control characters and terminal escape sequences. The host renders only its own labels and sanitized values.
- Allow once covers the next call. Allow always rewrites `permissions.toml` with `toml_edit`, preserves comments and unrelated grants, re-reads the file before merging, and writes atomically. A failed write falls back to allow once with a warning.
- On a request hash mismatch the prompt lists the added or widened requests. Allow always stores the current request hash and removes other changed grants so the plugin must be re-approved one capability at a time.
- One prompt is active at a time, at most eight wait in the queue, duplicates are collapsed, and an unanswered prompt expires after 30 seconds as a denial.

## Failure behavior

Missing capability:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "error": {
    "code": -32012,
    "message": "capability denied: fs.write",
    "data": {
      "capability": "fs.write",
      "scope": {
        "paths": ["$WORKSPACE/src"]
      }
    }
  }
}
```

`-32011` is already `INVALID_POSITION`. Reusing it would make errors ambiguous, so `CAPABILITY_DENIED` is a new additive code, `-32012`.

An unmet request:

- Produces a startup warning naming the plugin and the capability.
- Appears in `kawi doctor`.
- Appears in `:plugin list`.
- Does not prevent the plugin from running with its granted subset.

The first denial for each plugin and capability records a log entry with the missing capability and the TOML grant to add. Later denials increment a counter instead of flooding `:plugin logs`. `:plugin list` and `kawi doctor` keep showing missing grants and the permissions lines after a prompt is dismissed. Error data contains only the missing capability and its requested scope, never the effective grants or editor state.

Grant checks read a swappable registry, not a grant snapshot captured at spawn. Edit `permissions.toml` or run `:plugin reload` to change host-method and event grants immediately. Kawi restarts a plugin when its filesystem or execute scope changes so Landlock re-applies the ruleset.

`ui.message` is throttled to one message per plugin per second. Throttled messages are dropped and counted in `:plugin logs`.

A malformed `permissions.toml` during `:plugin reload` is fail-closed. Kawi swaps in the default tier and warns instead of keeping the previous grants or granting everything. The registry swap is one atomic replacement, so no call sees a partially loaded grant set.

## Advisory status

Process plugins call the host for editor and UI methods, and the kernel sandbox covers direct operating system access where the platform supports it:

- Enforced everywhere: `editor.read`, `editor.write`, `ui.message`, `diagnostics.read`, `diagnostics.publish`.
- Enforced on Linux with Landlock: `fs.read`, `fs.write`, and the execute half of `process.spawn`.
- Enforced for WebAssembly: `net.connect`, including DNS pinning and non-public address rejection.
- Advisory for process plugins: `net.connect` on every platform, `fs.*` and `process.spawn` off Linux, and every filesystem scope on older Landlock ABIs that cannot enforce rename or truncate restrictions.

Each spawned plugin receives a Landlock ruleset in the child between fork and exec, after `no_new_privs` is set. The ruleset is deny by default. The baseline read-only allowlist covers common runtime roots: `/usr`, `/lib`, `/lib64`, `/bin`, `/sbin`, `/opt`, `/nix/store`, `/run/current-system/sw`, the dynamic loader and ELF interpreter, runtime prefixes derived from the plugin executable (including virtualenv prefixes), `/etc/ld.so.cache`, TLS and DNS configuration, `/proc/self`, and `/dev/urandom`. `/dev/null` and the plugin directory are writable. A plugin that needs any other file must request `fs.read` or `fs.write`; it must request `process.spawn` to execute anything beyond the interpreter named in its manifest.

A granted path that does not exist is skipped with a log line instead of aborting the spawn or widening access. A granted executable that cannot be resolved is likewise skipped with a log line. A plugin that dies during startup logs the spawn error together with the sandbox status.

`process.spawn` grants are resolved to canonical executable paths at spawn time. Landlock allows `EXECUTE` on those paths, which closes the command-name race between grant checking and execution. Arguments are not scoped. `net.connect` cannot be enforced by Landlock because it scopes ports, not hostnames; WebAssembly plugins use the WASI socket policy instead.

The ruleset is best effort and follows the ABI the running kernel supports. Full enforcement requires an ABI that includes rename and truncate restrictions. If Landlock is unavailable the plugin starts unsandboxed. If the ABI is older, the plugin still starts under the partial ruleset. `:plugin list` marks a scope `enforced` only after a full ruleset was applied at spawn; unavailable and partial results stay `advisory`. A spawn fails visibly if an available ruleset cannot be applied.

Landlock is not a complete process boundary. It does not restrict hostnames or network destinations, so `net.connect` remains advisory for process plugins. Depending on the ABI it also may not stop connections to local sockets such as D-Bus or an ssh-agent, and it does not restrict signals or ptrace between same-user processes. Filesystem and executable scopes are enforced, but a process plugin remains same-user code. The WebAssembly runtime is the path to a complete boundary.

## Doctor and plugin list

`kawi doctor` reports:

- Parse errors in `permissions.toml`.
- Unknown plugins and stale grants.
- Grants for capabilities the plugin does not request.
- Requests without grants, including unresolved `$WORKSPACE`.
- Request hash mismatches, shown as changed with grants withheld.
- Subscriptions that cannot receive events.
- Advisory grants labeled `advisory` and fully enforced grants labeled `enforced`.

`:plugin list` shows effective, missing, stale, and changed capabilities without spawning the plugin. It also shows the first denial for each plugin and capability with the exact permissions line to add. Process filesystem and execute scopes are labeled `(enforced)` only after a spawn applied a full Landlock ruleset. WebAssembly `net.connect` is labeled `(enforced)`. Process `net.connect` is labeled `(advisory)`:

```text
gitsigns 0.2.0 ready
  caps: editor.read, editor.write, process.spawn (enforced), net.connect (advisory)
  ungranted: fs.read
```

## Migration

Enforcement is deny by default. A plugin with no `[capabilities]` table receives only the default tier. Existing plugins must add a request section and users must grant it. The bundled examples request the capabilities they use.

The grant file remains the durable consent surface; the prompt is the interactive path that writes it.

## Follow-ups

The remaining enforcement follow-up:

- Add WASI socket conformance fixtures for denied connects and DNS rebinding.

Other roadmap items:

- Re-approval review for several changed requests at once instead of one capability at a time.
