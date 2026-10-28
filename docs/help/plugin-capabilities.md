---
title = "Plugin capabilities"
aliases = ["plugin-capabilities", "permissions"]
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

- `$WORKSPACE`: the current project root
- `$PLUGIN_DIR`: the plugin directory
- `$CONFIG`: `$XDG_CONFIG_HOME/kawi`
- `$HOME`: the user home directory

Other variables are rejected. Expanded paths must be absolute. A grant path covers a requested path only when the grant path is equal to or inside the requested path. A grant never widens a request.

A path grants that subtree. If an existing path is a file, the grant covers only that file. Kawi resolves symlinks and rejects `..` segments before enforcement.

### Commands

`process.spawn` takes `commands`, an array of executable names or absolute paths. Kawi resolves names with the host PATH and compares resolved absolute paths. A grant entry covers only the same resolved executable. Arguments are not scoped.

### Hosts

`net.connect` takes `hosts`, an array of exact `host:port` entries. IPv6 uses brackets. Hostnames are lowercased. Wildcards and port ranges are not supported in version 1.

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
deny = ["ui.message"]
request_hash = "sha256:6f0f2b6e..."
```

`grant` is a table with the same string-or-table shape as the manifest request. This keeps the format valid TOML and avoids heterogeneous arrays.

Rules:

- The grant file lives in user configuration. Kawi never reads a grant file from a plugin directory and warns when one is present there.
- Every entry in the `grant` table must be a request in the manifest. Unrequested grants warn and are ignored.
- Grants for unknown plugins warn and are ignored.
- A scoped grant must be equal to or narrower than the request scope.
- Multiple grants for one capability are combined.
- A missing grant file grants only the default tier.
- A malformed grant file grants only the default tier and produces a startup warning.
- Kawi warns when the grant file is writable by group or other users.

`request_hash` is optional. When present, it is the SHA-256 of the canonical request set. Canonicalization sorts capability names and scope arrays, normalizes paths and hostnames, and serializes compact JSON. A mismatch warns that the plugin request changed. The hash is an audit signal, not consent. Changed requests remain ungranted until the user updates the grant file.

## Effective grant

The effective grant for a plugin is:

1. The default tier, currently `ui.message`, unless the plugin entry sets `deny = ["ui.message"]`. Only default-tier capabilities may be denied.
2. The explicit `grant` table for that plugin.
3. Intersected with the manifest request set.

Request and grant are independent. A plugin update that adds a request never gains access because the new request is outside the existing grant. The plugin starts with the intersection and warns about the rest. Kawi stores no mutable grant state inside the plugin directory.

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

Kawi checks that a method exists before checking a capability. Unknown methods return `-32601`. Known methods without an effective grant return `CAPABILITY_DENIED`.

Events are filtered before they enter the plugin queue. A plugin without `editor.read` receives no editor events, even if it subscribes to them. A plugin without `diagnostics.read` receives no diagnostic events. Startup and doctor warn when a subscription is unusable because of a missing capability.

The existing `host_capabilities` field in `initialize` remains method negotiation. Kawi advertises only methods covered by the effective grant. Capability enforcement is independent and still applies.

## Failure behavior

Missing capability:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "error": {
    "code": -32012,
    "message": "capability denied: editor.write",
    "data": {
      "capability": "editor.write"
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

## Advisory status

For process plugins, capabilities are enforced only when the plugin calls Kawi:

- Enforced: `editor.read`, `editor.write`, `ui.message`, `diagnostics.read`, `diagnostics.publish`.
- Advisory: `fs.read`, `fs.write`, `process.spawn`, `net.connect`.

A process plugin can call the operating system directly, so Kawi cannot enforce filesystem, network, or process scopes for it. `:plugin list` and `kawi doctor` label advisory grants as `advisory`, not as a sandbox.

Landlock will make `fs.read` and `fs.write` real for process plugins on Linux. Network and process scopes require seccomp, WebAssembly, or another sandbox. WebAssembly plugins will enforce every capability through host imports.

## Doctor and plugin list

`kawi doctor` reports:

- Parse errors in `permissions.toml`.
- Unknown plugins and capabilities in grants.
- Requests without grants.
- Request hash mismatches.
- Subscriptions that cannot receive events.
- Advisory grants for process plugins.

`:plugin list` shows effective and missing capabilities:

```text
gitsigns 0.2.0 ready
  caps: editor.read, editor.write, process.spawn (advisory)
  ungranted: fs.read
```

## Migration

Enforcement is deny by default. A plugin with no `[capabilities]` table receives only the default tier. Existing plugins must add a request section and users must grant it. The bundled examples will be updated with their requests when enforcement ships.

Interactive grant prompts are out of scope for version 1. The grant file is the consent surface.

## Roadmap

- Landlock for enforced filesystem scopes on Linux.
- WebAssembly with the component model and WIT interfaces for all scopes.
- Plugin-published diagnostics under `diagnostics.publish`, scoped by plugin name.
- Interactive grant review in the TUI.
