---
title = "Plugin capabilities"
aliases = ["permissions"]
related = ["plugins", "plugin-protocol", "configuration", "troubleshooting"]
---
# Plugin capabilities

Capabilities define what a plugin may ask Kawi to do. The plugin requests capabilities in its manifest. The user grants capabilities in a separate file that the plugin does not control. Kawi enforces the intersection and denies everything else.

This document specifies the permission model before enforcement ships. Process plugins remain trusted code. Until Landlock or WebAssembly enforcement is available, filesystem, network, and process capabilities are advisory for process plugins.

## Capability list

| Capability | Scope | Notes |
| --- | --- | --- |
| `editor.read` | none | Buffer text, metadata, cursor, selection, mode, revisions, and editor events |
| `editor.write` | none | `editor.apply_edit` and future buffer mutations |
| `ui.message` | none | Status messages. Default tier |
| `diagnostics.read` | none | Host diagnostic events |
| `diagnostics.publish` | none | Reserved for plugin-published diagnostics |
| `fs.read` | paths | Read files under the granted paths |
| `fs.write` | paths | Write files under the granted paths |
| `process.spawn` | commands | Spawn the granted executables |
| `net.connect` | hosts | Open outbound TCP connections to the granted hosts |

Names are stable. Unknown capabilities are ignored with a warning. New capabilities are additive and bump only the protocol minor version.

## Scopes

Unscoped capabilities use `true` in both requests and grants. Scoped capabilities use tables.

### Paths

`fs.read` and `fs.write` take `paths`, an array of directories or files. Paths may start with these variables:

- `$WORKSPACE`: the active workspace roots
- `$PLUGIN_DIR`: the plugin directory
- `$CONFIG`: `$XDG_CONFIG_HOME/kawi`
- `$HOME`: the user home directory

Other variables are rejected. Kawi expands variables for each enforcement call, so `$WORKSPACE` follows the active workspace. If no workspace is open, `$WORKSPACE` is unresolved: a requested `$WORKSPACE` is ungranted and a granted `$WORKSPACE` covers nothing. If several workspace roots are active, `$WORKSPACE` expands to each root.

Expanded paths must be absolute. Kawi rejects `..` components before resolving. Grant and requested paths are canonicalized for each call and symlinks are resolved before comparison, so a symlink that escapes a grant does not match. For a write target that does not exist, Kawi canonicalizes the nearest existing ancestor and appends the remaining components lexically.

Path scope is a component prefix. `/home/u/proj/src` is narrower than `/home/u/proj`. `/home/u/projection` is not inside `/home/u/proj`. A trailing separator is ignored. A grant for a file covers exactly that file. A grant never widens a request.

### Commands

`process.spawn` takes `commands`, an array of executable names or absolute paths. Kawi resolves bare names with the host PATH at the time of the call. Request and grant entries are both resolved and canonicalized to absolute executable paths. A grant covers only the same canonical path. Matching is never by name alone. A bare grant for `git` does not cover a request for an absolute path that resolves elsewhere. An executable that cannot be resolved is denied. Arguments are not scoped.

### Hosts

`net.connect` takes `hosts`, an array of exact `host:port` entries. IPv6 uses brackets. Kawi lowercases hostnames before comparison. Matching is exact. `api.example.com:443` is not covered by a grant for `*.example.com:443`, and wildcards are invalid in version 1. Port ranges are not supported.

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

Reordering a manifest does not change the hash. Widening a scope does. A mismatch warns that the plugin request changed. The hash is an audit signal, not consent. Changed requests remain ungranted until the user updates the grant file.

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

Events are filtered before they enter the plugin queue and are checked against the current grants on every delivery. A plugin without `editor.read` receives no editor events, even if it subscribes to them. A plugin without `diagnostics.read` receives no diagnostic events. Startup and doctor warn when a subscription is unusable because of a missing capability.

The `host_capabilities` and `events` fields in `initialize` are derived from the same table as enforcement. Kawi advertises only methods and events covered by the effective grant at initialization time. Enforcement continues to use the current grants, so revocation applies to the next message without restarting the plugin.

The `invoke_command` context is editor state. Without `editor.read`, Kawi replaces it with an empty context: buffer and revision `0`, no path or language, normal mode, cursor at the origin, and no selection. Command arguments are caller-supplied command input from configuration or keybindings, not editor state, and are passed through.

Interactive grant prompts are not implemented yet. Denial records use the exact TOML that a later prompt can reuse.

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

The first denial for each plugin and capability records a log entry with the missing capability and the TOML grant to add. Later denials increment a counter instead of flooding `:plugin logs`. Error data contains only the missing capability and its requested scope, never the effective grants or editor state.

Grant checks read a swappable registry, not a grant snapshot captured at spawn. Edit `permissions.toml` and run `:plugin reload` to revoke or narrow grants without restarting plugins. The next host call or event delivery uses the new grants.

`ui.message` is throttled to one message per plugin per second. Throttled messages are dropped and the first one is recorded in `:plugin logs`.

## Advisory status

For process plugins, capabilities are enforced only when the plugin calls Kawi:

- Enforced: `editor.read`, `editor.write`, `ui.message`, `diagnostics.read`, `diagnostics.publish`.
- Advisory: `fs.read`, `fs.write`, `process.spawn`, `net.connect`.

A process plugin can call the operating system directly, so Kawi cannot enforce filesystem, network, or process scopes for it. `:plugin list` and `kawi doctor` label advisory grants as `advisory`, not as a sandbox.

Landlock will make `fs.read` and `fs.write` real for process plugins on Linux. Network and process scopes require seccomp, WebAssembly, or another sandbox. WebAssembly plugins will enforce every capability through host imports.

## Doctor and plugin list

`kawi doctor` reports:

- Parse errors in `permissions.toml`.
- Unknown plugins and stale grants.
- Grants for capabilities the plugin does not request.
- Requests without grants, including unresolved `$WORKSPACE`.
- Request hash mismatches.
- Subscriptions that cannot receive events.
- Advisory grants for process plugins.

`:plugin list` shows effective, missing, and stale capabilities without spawning the plugin:

```text
gitsigns 0.2.0 ready
  caps: editor.read, editor.write, process.spawn (advisory)
  ungranted: fs.read
```

## Migration

Enforcement is deny by default. A plugin with no `[capabilities]` table receives only the default tier. Existing plugins must add a request section and users must grant it. The bundled examples will be updated with their requests when enforcement ships.

Interactive grant prompts are out of scope for version 1. The grant file is the consent surface.

## Follow-ups

These two issues are reserved for the first enforcement follow-ups:

- `plugin: enforce filesystem scopes with Landlock on Linux`
- `plugin: add WebAssembly component runtime`

Other roadmap items:

- Plugin-published diagnostics under `diagnostics.publish`, scoped by plugin name.
- Interactive grant review in the TUI.
