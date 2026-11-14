---
title = "Plugin host APIs"
aliases = ["permissions"]
related = ["plugins", "plugin-protocol", "plugin-authoring", "plugin-migration", "configuration", "troubleshooting"]
---
# Plugin host APIs

Plugins declare the host APIs they use in `plugin.toml`. Kawi enables every declared API for a managed plugin. There is no `permissions.toml` and no per-capability prompt. Installing or updating a plugin is the trust decision. Process plugins are frozen at protocol minor 12; the buffer, window, control, and options APIs are available only to WebAssembly components.

Declarations are a compatibility and audit surface. They tell the user what the plugin can do and let Kawi reject a component that imports an undeclared interface. The WebAssembly sandbox and the fixed WIT contract remain the enforcement boundary. A plugin cannot gain access by editing its installed files because the manifest and component hash are pinned in the lockfile.

## API list

| API | Scope | Meaning |
| --- | --- | --- |
| `editor.read` | none | Read editor state and receive editor events |
| `editor.write` | none | Apply text edits |
| `editor.buffers` | none | List and read buffers, including text and revisions |
| `editor.buffers.write` | `paths` | Open files and apply edits to the active buffer |
| `editor.windows` | none | List, inspect, focus, open, and close windows |
| `editor.control` | none | Set the active buffer cursor, selection, and mode |
| `options.read` | none | List and read editor options |
| `editor.execute` | `commands` | Execute the declared built-in editor commands |
| `editor.decorations` | none | Publish buffer highlights, virtual text, and gutter signs |
| `ui.message` | none | Show status messages |
| `ui.prompt` | none | Request text input, a selection, or confirmation |
| `ui.progress` | none | Show cancellable progress |
| `ui.statusline` | none | Publish status line segments |
| `ui.panel` | none | Open and control a docked text panel |
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
"editor.buffers" = true
"editor.buffers.write" = { paths = ["$WORKSPACE/src"] }
"editor.windows" = true
"editor.control" = true
"options.read" = true
"editor.execute" = { commands = ["edit.format", "file.save"] }
"editor.decorations" = true
"ui.message" = true
"ui.prompt" = true
"ui.progress" = true
"ui.statusline" = true
"ui.panel" = true
"fs.read" = { paths = ["$WORKSPACE/src"] }
"process.spawn" = { commands = ["git", "rg"] }
"net.connect" = { hosts = ["api.example.com:443"] }
```

## Paths

`fs.read`, `fs.write`, and `editor.buffers.write` take `paths`, an array of directories or files. Paths may use `$WORKSPACE`, `$PLUGIN_DIR`, `$CONFIG`, and `$HOME`. Other variables are rejected. Expanded paths must be absolute and must not contain `..`.

Path scope is a component prefix. `/home/u/proj/src` is narrower than `/home/u/proj`. `/home/u/projection` is not inside `/home/u/proj`. A trailing separator is ignored. WebAssembly preopens require directories, so a file path is skipped with a warning for WASM plugins. `editor.buffers.write` resolves and canonicalizes the target before opening or editing; a path outside the declared scope returns the capability error.

## Process commands

`process.spawn` takes `commands`, an array of executable names or absolute paths. Relative paths are rejected. Bare names are resolved with the host `PATH` when the sandbox is built. Arguments are not scoped.

`process.spawn` also enables background jobs. Process plugins use `process.open`, `process.write`, `process.close_stdin`, `process.signal`, `process.kill`, `process.status`, and `process.poll`. WASM plugins use the `process-jobs` WIT interface with the same operations. A job command must resolve to an entry in the same command scope. Jobs run in the plugin directory by default. A job `cwd` may be the plugin directory, the workspace, or the configuration directory; relative paths resolve from the plugin directory. Job environment entries add to or override the inherited environment. Each plugin may run 16 jobs, each job may emit 8 MiB of output, and each write may send 1 MiB. Kawi kills a plugin's jobs when the plugin stops, restarts, reloads, or is disabled. Process plugins receive job events as notifications; WASM plugins drain queued events with `process.poll`. See [Plugin protocol](help:plugin-protocol) for the job methods and events.

The WebAssembly runtime exposes the `process-run` one-shot interface and the `process-jobs` interface. Both use the same `process.spawn` command scope. WASM job events are queued because components are passive; call `poll` on the next invocation to drain them.

## Editor commands

`editor.execute` takes `commands`, an array of built-in editor command IDs. Command IDs are lowercase and may contain dots, hyphens, and underscores. The list must not be empty and wildcards are not supported. Scope entries use canonical IDs such as `file.save` and `edit.format`; aliases are accepted at call time only when the canonical ID is allowed.

The plugin can call `editor.list_commands` to discover the allowed commands and `editor.execute` to run one. Listing returns canonical IDs, descriptions, and aliases. The scope is an allowlist: a command outside the list returns `-32012`. Plugin-defined commands, `keys.send`, and commands that require arguments are not composable and return `-32602`.

The editor runs the command synchronously before responding. Changes made by the command carry the `plugin:<name>` change origin.

## Visual customization

`editor.decorations` enables `editor.set_decorations` and the WASM `editor-decorations` and `editor-signs` interfaces. Visual decorations and signs are stored independently, so updating one does not clear the other. The process API uses `scope` to select `all`, `visual`, or `signs`; it defaults to `all`. Decorations are data: the host owns layout, colors, and rendering. They remain until the plugin replaces them, the buffer closes, or the plugin stops. A revision change can leave them briefly stale until the next publish.

Plugins declare named decoration groups in `plugin.toml`:

```toml
[[decorations]]
id = "todo"
color = "info"
fill = "both"
italic = true

[[decorations]]
id = "todo_text"
color = "info"
fill = "foreground"
italic = true
```

Group IDs are local to the plugin and may not use the built-in style names `accent`, `info`, `success`, `warning`, `error`, or `muted`. `color` selects the semantic base color. `fill` accepts `both`, `foreground`, or `background`. `hidden`, `bold`, `italic`, and `underline` set text attributes. Hidden text is revealed when the cursor is on it.

```json
{
  "buffer": 1,
  "revision": 12,
  "scope": "all",
  "decorations": [
    {
      "kind": "highlight",
      "start": {"line": 4, "byte_column": 0},
      "end": {"line": 4, "byte_column": 8},
      "group": "todo"
    },
    {
      "kind": "virtual_text",
      "line": 4,
      "text": "indexed",
      "group": "todo_text"
    },
    {
      "kind": "sign",
      "line": 4,
      "text": "!",
      "group": "todo"
    }
  ]
}
```

Each decoration sets exactly one of `group` or `style`. `group` must be declared by the plugin. `style` is the legacy built-in fallback and accepts `accent`, `info`, `success`, `warning`, `error`, or `muted`. A sign must occupy exactly one terminal cell. Highlights must be ordered and inside the buffer. Kawi accepts at most 10,000 decorations and 512 bytes of text per virtual text or sign. Diagnostics outrank plugin signs, and plugin signs outrank Git signs.

Themes customize each group with `[plugin_decorations.<plugin>.<group>]`. Overrides support `foreground`, `background`, `fill`, `bold`, `italic`, and `underline`. See [Themes](help:themes).

`ui.statusline` enables `ui.set_statusline`. A plugin replaces all of its status line segments. Segments are shown where the `plugins` element appears in `[ui.statusline]` and are ordered by descending `priority`, then by plugin name.

```json
{
  "segments": [
    {"text": "indexed 12/12", "style": "success", "priority": 10},
    {"text": "lint", "style": "warning", "priority": 5}
  ]
}
```

Kawi accepts at most 16 segments and 256 bytes of text per segment. Control characters and terminal escapes are removed before display.

## Interactive UI

`ui.prompt` enables `ui.input`, `ui.select`, and `ui.confirm`. Each call waits for a user response and returns an outcome of `text`, `selected`, `confirmed`, or `canceled`. Only one prompt can be active at a time across all plugins. A second prompt returns `-32040`.

`ui.progress` enables `ui.progress_start`, `ui.progress_update`, and `ui.progress_finish`. Progress is displayed in the status area. The user cancels with `:plugin cancel` or a key binding for `plugin.cancel`; the next `ui.progress_update` returns `{"canceled": true}`. Only one progress can be active at a time across all plugins. A second progress returns `-32040`.

Prompt calls do not count against the invocation timeout. Progress calls do. Stopping a plugin clears its prompt and progress and fails a pending prompt with `-32030`.

## Plugin panels

`ui.panel` enables `ui.panel_open`, `ui.panel_update`, `ui.panel_focus`, and `ui.panel_close`. A panel is a read-only, styled text surface docked above the task panel. Only one panel can be active across all plugins. A second open returns `-32040`. A panel is removed when its plugin stops, restarts, reloads, or is disabled.

Panels accept at most 1000 lines, 4096 bytes per line, 1 MiB of total line text, a 256-byte title, and a 256-byte status. Lines render with an optional `accent`, `info`, `success`, `warning`, `error`, or `muted` style. Control characters and terminal escapes are removed before display.

The host owns layout, focus, scrolling, and closing. `height` is clamped to 4 through 18 and defaults to 8. The panel reserves at least four editor rows and hides when the editor is too small. Focus moves with `ui.panel_focus` or by clicking the panel. While focused, the host handles scrolling and closing: `Esc` returns focus to the editor and `q` closes the panel. The `plugin.panel` command toggles focus, and `:plugin panel close` closes the panel. There is no raw key input API.

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

Users can edit settings through `:plugin settings <plugin>`. The editor validates values against the declared type, choices, and range, preserves existing file formatting, and never writes a value that the manifest rejects. Ctrl+R removes the override and restores the default.

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
- Host-mediated editor, UI, and diagnostics calls require the matching declaration. Interactive prompts also require an integer JSON-RPC request id. `config.get` and `config.list` need no declaration because they only expose the plugin's own settings.
- `editor.execute` calls are limited to the declared command allowlist. Only built-in commands that take no arguments are composable.
- Events require the matching declaration. A plugin that does not declare `editor.read` receives no editor events.
- Process plugins are development-only trusted code. On Linux with a full Landlock ABI, Kawi applies filesystem and process-execution scopes from the declarations. macOS has no Landlock, so process plugins run unsandboxed there. Network scope is advisory for process plugins.

## Updates

Kawi compares the declared API surface when you run `:plugin update`. Added or widened APIs require one confirmation that shows the new commit and the changed APIs. Removed or narrowed APIs do not prompt. A component hash change without an API change updates without a prompt.

## Doctor and plugin list

`:plugin list` shows each plugin's declared APIs without starting it. `kawi doctor` checks installed plugin directories, the lockfile, and development plugins. It reports missing installed directories, malformed lockfiles, and plugin manifest warnings.

## Development plugins

Development plugins are not in the lockfile and are not managed by `:plugin install`, `:plugin update`, `:plugin disable`, or `:plugin remove`. They are loaded from `$XDG_CONFIG_HOME/kawi/plugins` with a startup warning. Remove or edit them directly.
