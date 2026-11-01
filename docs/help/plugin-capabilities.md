---
title = "Plugin host APIs"
aliases = ["permissions"]
related = ["plugins", "plugin-protocol", "plugin-authoring", "plugin-migration", "configuration", "troubleshooting"]
---
# Plugin host APIs

Plugins declare the host APIs they use in `plugin.toml`. Kawi enables every declared API for a managed plugin. There is no `permissions.toml` and no per-capability prompt. Installing or updating a plugin is the trust decision.

Declarations are a compatibility and audit surface. They tell the user what the plugin can do and let Kawi reject a component that imports an undeclared interface. The WebAssembly sandbox and the fixed WIT contract remain the enforcement boundary. A plugin cannot gain access by editing its installed files because the manifest and component hash are pinned in the lockfile.

## API list

| API | Scope | Meaning |
| --- | --- | --- |
| `editor.read` | none | Read editor state and receive editor events |
| `editor.write` | none | Apply text edits |
| `editor.execute` | `commands` | Execute the declared built-in editor commands |
| `ui.message` | none | Show status messages |
| `diagnostics.read` | none | Receive diagnostics events |
| `diagnostics.publish` | none | Publish diagnostics for the current buffer |
| `fs.read` | `paths` | Read files under the declared paths |
| `fs.write` | `paths` | Write files under the declared paths |
| `process.spawn` | `commands` | Spawn the declared executables |
| `net.connect` | `hosts` | Open outbound TCP connections to the declared hosts |

Unscoped APIs use `true`. Scoped APIs use tables:

```toml
[capabilities]
"editor.read" = true
"editor.write" = true
"editor.execute" = { commands = ["edit.format", "file.save"] }
"ui.message" = true
"fs.read" = { paths = ["$WORKSPACE/src"] }
"process.spawn" = { commands = ["git", "rg"] }
"net.connect" = { hosts = ["api.example.com:443"] }
```

## Paths

`fs.read` and `fs.write` take `paths`, an array of directories or files. Paths may use `$WORKSPACE`, `$PLUGIN_DIR`, `$CONFIG`, and `$HOME`. Other variables are rejected. Expanded paths must be absolute and must not contain `..`.

Path scope is a component prefix. `/home/u/proj/src` is narrower than `/home/u/proj`. `/home/u/projection` is not inside `/home/u/proj`. A trailing separator is ignored. WebAssembly preopens require directories, so a file path is skipped with a warning for WASM plugins.

## Process commands

`process.spawn` takes `commands`, an array of executable names or absolute paths. Relative paths are rejected. Bare names are resolved with the host `PATH` when the sandbox is built. Arguments are not scoped.

## Editor commands

`editor.execute` takes `commands`, an array of built-in editor command IDs. Command IDs are lowercase and may contain dots, hyphens, and underscores. The list must not be empty and wildcards are not supported. Scope entries use canonical IDs such as `file.save` and `edit.format`; aliases are accepted at call time only when the canonical ID is allowed.

The plugin can call `editor.list_commands` to discover the allowed commands and `editor.execute` to run one. Listing returns canonical IDs, descriptions, and aliases. The scope is an allowlist: a command outside the list returns `-32012`. Plugin-defined commands, `keys.send`, and commands that require arguments are not composable and return `-32602`.

The editor runs the command synchronously before responding. Changes made by the command carry the `plugin:<name>` change origin.

## Settings

Plugins declare their settings in `plugin.toml`:

```toml
[settings.max_items]
type = "integer"
default = 10
description = "Maximum number of items"
minimum = 1
maximum = 100

[settings.mode]
type = "string"
default = "auto"
description = "Display mode"
enum = ["auto", "light", "dark"]
```

Supported types are `boolean`, `integer`, `number`, and `string`. `enum` is available for strings. `minimum` and `maximum` are available for integers and numbers. A default must match its type and satisfy its choices and range.

Users override settings in `$XDG_CONFIG_HOME/kawi/plugins.toml`:

```toml
[example]
max_items = 25
mode = "dark"
```

`config.get` and `config.list` expose the effective values for the calling plugin. They need no capability declaration because they only read that plugin's own namespace. Missing keys return `null`. Invalid values and unknown keys are logged, and invalid values fall back to the manifest default. Kawi reads the file on demand, so edits apply without restarting the plugin.

## Hosts

`net.connect` takes `hosts`, an array of exact `host:port` entries. IPv6 uses brackets. Hostnames are lowercased before comparison. Wildcards and port ranges are not supported.

The WebAssembly runtime resolves declared hostnames once per session with a five-second cap and pins the resulting public addresses. Loopback, private, link-local, shared, documentation, benchmarking, multicast, reserved, 6to4, Teredo, ORCHID, and IPv4-mapped answers are rejected unless the declaration names a connectable address literally. The session never re-resolves. Restart builds a new session and resolves again. WASI name lookup is filtered to declared hostnames. Only outbound TCP connect is permitted.

## Command gates

A command may declare `capabilities = ["editor.read"]` to restrict invocation. The gate must be a subset of the plugin declarations. It does not add access. Every host call is still checked against the declared APIs.

```toml
[[commands]]
id = "format"
title = "Format buffer"
capabilities = ["editor.write"]
```

An ungated command runs whenever the plugin runs.

## Enforcement

- WebAssembly plugins can only import interfaces declared in the manifest. An undeclared import fails before instantiation.
- WASI filesystem preopens come only from declared `fs.read` and `fs.write` paths.
- WASI network access comes only from declared `net.connect` hosts and is limited to outbound TCP connect.
- Host-mediated editor, UI, and diagnostics calls require the matching declaration. `config.get` and `config.list` need no declaration because they only expose the plugin's own settings.
- `editor.execute` calls are limited to the declared command allowlist. Only built-in commands that take no arguments are composable.
- Events require the matching declaration. A plugin that does not declare `editor.read` receives no editor events.
- Process plugins are development-only trusted code. On Linux with a full Landlock ABI, Kawi applies filesystem and process-execution scopes from the declarations. Network scope is advisory for process plugins.

## Updates

Kawi compares the declared API surface when you run `:plugin update`. Added or widened APIs require one confirmation that shows the new commit and the changed APIs. Removed or narrowed APIs do not prompt. A component hash change without an API change updates without a prompt.

## Doctor and plugin list

`:plugin list` shows each plugin's declared APIs without starting it. `kawi doctor` checks installed plugin directories, the lockfile, and development plugins. It reports missing installed directories, malformed lockfiles, and plugin manifest warnings.

## Development plugins

Development plugins are not in the lockfile and are not managed by `:plugin install`, `:plugin update`, `:plugin disable`, or `:plugin remove`. They are loaded from `$XDG_CONFIG_HOME/kawi/plugins` with a startup warning. Remove or edit them directly.
