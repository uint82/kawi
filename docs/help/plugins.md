---
title = "Plugins"
aliases = ["plugin"]
related = ["plugin-protocol", "plugin-capabilities", "plugin-authoring", "plugin-migration", "keymaps", "configuration", "troubleshooting"]
---
# Plugins

Kawi installs managed plugins from GitHub releases. A managed plugin is a WebAssembly component with a versioned manifest. Kawi verifies the release artifact, pins the resolved commit and checksum, and enables the plugin after one confirmation. There is no permission file and no per-capability prompt.

Local plugins in `$XDG_CONFIG_HOME/kawi/plugins` are a separate development mode. They are loaded with a warning and are never updated or removed by `:plugin`. Process plugins remain available only in this development mode and run as trusted code with your full user permissions.

Installed plugins live in `$XDG_DATA_HOME/kawi/plugins`, normally `~/.local/share/kawi/plugins`. The lockfile lives in `$XDG_CONFIG_HOME/kawi/plugins.lock`, normally `~/.config/kawi/plugins.lock`. Plugin settings live in `$XDG_CONFIG_HOME/kawi/plugins.toml`, normally `~/.config/kawi/plugins.toml`. Kawi never loads plugins from a project directory.

## Install

```text
:plugin install owner/repo
:plugin install owner/repo@v1.2.3
```

The source may also be a GitHub URL. Without a reference, Kawi uses the latest release. Kawi resolves the release tag to a commit, reads `plugin.toml` at that commit, checks the plugin API version, and downloads the prebuilt `.wasm` release asset named by `component`.

Before installing, Kawi shows one confirmation with the repository, author, commit, API version, checksum, and declared host APIs. Enter installs and enables the plugin. Esc cancels.

The release must contain:

- `plugin.toml` at the repository root for the release commit.
- A release asset whose file name matches the manifest `component` file name.
- `runtime = "wasm"`.
- `protocol` matching the Kawi plugin API version.
- `sha256` matching the component bytes.

Kawi downloads into a staging directory, verifies the checksum, validates the component, and only then moves it into place. A failed download or validation leaves the previous installation untouched.

## Lockfile

Kawi pins each managed plugin in `plugins.lock`:

```toml
version = 1

[plugins.echo]
source = "owner/repo"
reference = "v1.2.3"
commit = "0123456789abcdef0123456789abcdef01234567"
sha256 = "sha256:..."
api = 1
enabled = true
```

The lockfile is machine-managed. `reference` is omitted when the plugin was installed from the latest release. The commit and checksum are the identity used for updates.

On startup Kawi ignores an installed plugin that is missing from the lockfile, or whose component hash or API version does not match the locked values. A corrupt lockfile blocks `:plugin` mutations until you fix or remove it.

## Commands

- `:plugin install <owner/repo[@ref]>` resolves, verifies, confirms, installs, and enables a managed plugin.
- `:plugin update <plugin>` resolves the configured source again. If the commit is unchanged, Kawi reports that the plugin is up to date. If the API surface is unchanged, Kawi updates immediately and reports the new commit. If a host API was added or widened, Kawi shows one confirmation with the new commit and the changed APIs.
- `:plugin disable <plugin>` keeps the plugin installed but stops loading it.
- `:plugin enable <plugin>` loads a disabled plugin again.
- `:plugin remove <plugin>` confirms, removes the installed directory, and removes the lockfile entry.
- `:plugin list` shows installed and development plugins, versions, runtimes, status, source, commit, declared host APIs, and commands.
- `:plugin reload` reloads manifests from disk.
- `:plugin restart [plugin]` restarts one plugin or every plugin.
- `:plugin logs [plugin]` shows plugin logs.
- `:plugin cancel` cancels the active plugin progress.
- `:plugin inspect <plugin>` shows the manifest, declared host APIs, settings, component hash, and WebAssembly imports.
- `:plugin settings [plugin]` opens an editor for declared settings. Without a plugin, Kawi shows a plugin chooser. Enter edits or toggles a value, Ctrl+R resets a setting to its default, and Esc closes the editor. Changes are validated against the manifest and written to `plugins.toml` without removing comments or unrelated tables.

Use `:commands` to discover plugin command names. Plugin commands appear with a `[plugin: name]` marker.

## Host APIs

Plugins declare the host APIs they use in `plugin.toml`:

```toml
[capabilities]
"editor.read" = true
"editor.write" = true
"editor.execute" = { commands = ["edit.format", "file.save"] }
"editor.decorations" = true
"ui.message" = true
"ui.prompt" = true
"ui.progress" = true
"ui.statusline" = true
```

Declarations are a compatibility and audit surface, not a user grant. Kawi enables every declared API for a managed plugin. The WebAssembly component can only import interfaces that it declared. A component that imports an undeclared interface fails to load. A plugin cannot gain access by editing its installed files because the manifest and component hash are pinned in the lockfile.

Available host APIs are `editor.read`, `editor.write`, `editor.execute`, `editor.decorations`, `ui.message`, `ui.prompt`, `ui.progress`, `ui.statusline`, `diagnostics.read`, `diagnostics.publish`, `fs.read`, `fs.write`, `process.spawn`, and `net.connect`. Scoped APIs use `paths`, `commands`, or `hosts` tables. See [Plugin host APIs](help:plugin-capabilities) for scope syntax and enforcement.

Plugins can declare settings and read their effective values without a capability declaration:

```toml
[settings.max_items]
type = "integer"
default = 10
minimum = 1
maximum = 100
```

User values live in `plugins.toml` under the plugin name:

```toml
[echo]
max_items = 25
```

Invalid values fall back to the manifest default and are logged. Settings are read on demand, so changes apply without restarting the plugin. Use `:plugin settings <plugin>` to edit the same values with type-aware validation.

## Manifest

```toml
name = "echo"
version = "0.1.0"
protocol = 1
runtime = "wasm"
description = "Example plugin"
component = "echo.wasm"
sha256 = "sha256:..."
timeout_ms = 5000
subscribes = ["file_saved"]

[capabilities]
"ui.message" = true

[[activation_events]]
event = "on_command"

[[commands]]
id = "ping"
title = "Ping the plugin"

[commands.keybindings]
normal = ["<leader>pp"]
```

The plugin name is the namespace. The command above is `echo.ping`, and you can bind it in `keymaps.toml` like any built-in command.

- `component` is a path relative to the plugin directory. Managed releases use the file name as the release asset name.
- `sha256` is required for WASM plugins. `kawi plugin build` updates it.
- `timeout_ms` applies to initialization and each invocation. The default is `5000`. The range is `100` to `60000`.
- `capabilities` declares host APIs and command gates. A command gate must be a subset of the plugin declarations.
- `subscribes` lists events delivered to a running plugin. Delivery never starts a plugin.
- `debounce` optionally delays an event per subscription.
- `activation_events` lists `on_startup`, `on_command`, and `on_file_open` with optional `filetypes` and `globs`.

## Lifecycle

1. Kawi reads installed and development manifests and registers static commands and keybindings.
2. The first invocation or activation event starts the plugin.
3. Kawi validates the component hash and imports before instantiation.
4. Kawi calls `init` with the host protocol version.
5. Kawi sends `invoke_command` or events.
6. A crashed plugin restarts on the next invocation with exponential backoff. Five consecutive failures disable it until `:plugin restart`.

Kawi supports one in-flight invocation per plugin. Invocations that arrive while another command is running enter a bounded FIFO queue. The queue holds four commands for ten seconds by default. A full queue rejects the command and logs a rejection. An expired queued command is dropped and logged.

## Events

Kawi sends `buffer_changed`, `buffer_opened`, `file_opened`, `buffer_closed`, `file_saved`, `mode_changed`, and `diagnostics_changed`. Events are notifications and never require a response.

- Delivery is separate from activation. A stopped plugin receives nothing and is not started by an event.
- Each plugin has a bounded queue. Buffer changes are coalesced per buffer, optional debounce applies, and other events are dropped with a log line when the queue is full.
- `buffer_changed` carries the buffer identifier, revision, previous revision, origin, UTF-8 byte edits, and a `full_resync` flag. Apply edits in order, or call `editor.get_state` when the revision does not match.
- Origins are `user`, `undo`, `plugin:<name>`, `external_reload`, and `mixed`. A plugin does not receive its own edits.
- `:plugin logs <plugin>` reports queue overflows. An overflow forces a resync on the next delivered change.

Kawi sends diagnostics from LSP and tasks to plugins. A plugin with `diagnostics.publish` can publish diagnostics for the current named buffer. Kawi prefixes the source with the plugin name, so plugins cannot spoof another provider. Publishing an empty list clears that plugin's diagnostics for the buffer.

## Host methods

- `editor.get_state` returns the buffer identifier, revision, path, language, mode, cursor, selection, and optionally the full text.
- `editor.apply_edit` applies text edits only when the supplied buffer identifier and revision match the current buffer. Stale edits are rejected and the buffer is unchanged.
- `editor.execute` runs a built-in editor command from the declared allowlist. Plugin commands and commands that require arguments are rejected.
- `editor.list_commands` returns the allowed canonical command IDs, descriptions, and aliases.
- `editor.set_diagnostics` replaces the plugin-published diagnostics for the current buffer. The buffer identifier and revision must match, and the buffer must have a file path.
- `editor.set_decorations` replaces the plugin's highlights, virtual text, and gutter signs for one buffer and revision.
- `ui.set_statusline` replaces the plugin's status line segments.
- `ui.message` shows a status message.
- `config.get` returns one effective setting value or `null`.
- `config.list` returns every effective setting for the plugin, sorted by key.

Positions are zero-based and use line plus UTF-8 byte column. Kawi rejects byte columns inside a code point.

See [Plugin protocol](help:plugin-protocol) for the process development protocol and [Writing a WASM plugin](help:plugin-authoring) for the component contract.

## Development plugins

Copy a plugin directory into `$XDG_CONFIG_HOME/kawi/plugins` to run it locally. Kawi prints a startup warning and marks the plugin as `development` in `:plugin list`. Development plugins can be process plugins or WASM plugins. They are trusted local code, they are not pinned in the lockfile, and `:plugin disable` and `:plugin remove` do not manage them.

The repository includes two Python examples and a WASM example:

- `examples/plugins/echo` registers `echo.ping`, `echo.hello`, and `echo.retry`.
- `examples/plugins/trim` trims trailing whitespace after `file_saved` and through the `trim.trim` command.
- `examples/wasm-trim` is the same trim plugin on the WASM runtime.
