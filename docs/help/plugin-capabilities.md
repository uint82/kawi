---
title = "Plugin host APIs"
aliases = ["permissions"]
related = ["plugins", "plugin-protocol", "plugin-authoring", "plugin-migration", "configuration", "troubleshooting"]
---
# Plugin host APIs

Plugins declare the host APIs they use in `plugin.toml`. Kawi enables every declared API for a managed plugin. There is no `permissions.toml` and no per-capability prompt. Installing or updating a plugin is the trust decision. Process plugins are frozen at protocol minor 12; the buffer, window, control, option, runtime command, runtime keymap, autocmd, schedule, variable, overlay, picker, and task APIs are available only to WebAssembly components. The `window_created`, `window_closed`, `language_changed`, and `user` events are delivered to autocmds, not to `on-event` subscribers. An autocmd can register only for events allowed by the plugin's other capabilities; `editor.read` covers buffer and language events, `editor.windows` covers window events, and `editor.autocmds` covers user events.

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
| `options.write` | none | Set and reset editor options |
| `editor.execute` | `commands` | Execute the declared built-in editor commands |
| `editor.commands` | none | Register commands at runtime |
| `editor.keymaps` | none | Register key bindings at runtime |
| `editor.autocmds` | none | Register, remove, list, and clear asynchronous autocmds and emit user events |
| `editor.schedule` | none | Create, cancel, and list asynchronous timers |
| `editor.variables` | none | Store global, plugin, buffer, and window variables |
| `editor.decorations` | none | Publish buffer highlights, virtual text, and gutter signs |
| `ui.message` | none | Show status messages |
| `ui.prompt` | none | Request text input, a selection, or confirmation |
| `ui.progress` | none | Show cancellable progress |
| `ui.statusline` | none | Publish status line segments |
| `ui.panel` | none | Open and control a docked text panel |
| `ui.overlay` | none | Open a floating text overlay with keyboard and paste input |
| `ui.picker` | none | Show a native picker and wait for the selected item |
| `diagnostics.read` | none | Receive diagnostics events |
| `diagnostics.publish` | none | Publish diagnostics for the current buffer |
| `fs.read` | `paths` | Read files under the declared paths |
| `fs.write` | `paths` | Write files under the declared paths |
| `process.spawn` | `commands` | Spawn the declared executables |
| `task.run` | `tasks` | List and run the declared project tasks |
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
"options.write" = true
"editor.execute" = { commands = ["edit.format", "file.save"] }
"editor.commands" = true
"editor.keymaps" = true
"editor.autocmds" = true
"editor.schedule" = true
"editor.variables" = true
"editor.decorations" = true
"ui.message" = true
"ui.prompt" = true
"ui.progress" = true
"ui.statusline" = true
"ui.panel" = true
"ui.overlay" = true
"ui.picker" = true
"fs.read" = { paths = ["$WORKSPACE/src"] }
"process.spawn" = { commands = ["git", "rg"] }
"task.run" = { tasks = ["check", "test"] }
"net.connect" = { hosts = ["api.example.com:443"] }
```

## Paths

`fs.read`, `fs.write`, and `editor.buffers.write` take `paths`, an array of directories or files. Paths may use `$WORKSPACE`, `$PLUGIN_DIR`, `$CONFIG`, and `$HOME`. Other variables are rejected. Expanded paths must be absolute and must not contain `..`.

Path scope is a component prefix. `/home/u/proj/src` is narrower than `/home/u/proj`. `/home/u/projection` is not inside `/home/u/proj`. A trailing separator is ignored. WebAssembly preopens require directories, so a file path is skipped with a warning for WASM plugins. `editor.buffers.write` resolves and canonicalizes the target before opening or editing; a path outside the declared scope returns the capability error.

## Process commands

`process.spawn` takes `commands`, an array of executable names or absolute paths. Relative paths are rejected. Bare names are resolved with the host `PATH` when the sandbox is built. Arguments are not scoped.

`process.spawn` also enables background jobs. Process plugins use `process.open`, `process.write`, `process.close_stdin`, `process.signal`, `process.kill`, `process.status`, and `process.poll`. WASM plugins use the `process-jobs` WIT interface with the same operations. A job command must resolve to an entry in the same command scope. Jobs run in the plugin directory by default. A job `cwd` may be the plugin directory, the workspace, or the configuration directory; relative paths resolve from the plugin directory. Job environment entries add to or override the inherited environment. Each plugin may run 16 jobs, each job may emit 8 MiB of output, and each write may send 1 MiB. Kawi kills a plugin's jobs when the plugin stops, restarts, reloads, or is disabled. Process plugins receive job events as notifications; WASM plugins drain queued events with `process.poll`. See [Plugin protocol](help:plugin-protocol) for the job methods and events.

The WebAssembly runtime exposes the `process-run` one-shot interface and the `process-jobs` interface. Both use the same `process.spawn` command scope. WASM job events are queued because components are passive; call `poll` on the next invocation to drain them.

## Project tasks

`task.run` takes `tasks`, an array of task names from the project `tasks.toml` and the global task file. A plugin can list only the configured tasks in its scope and can start only those names. The scope is an exact allowlist; an unlisted name returns `-32012`, and a listed name that is not configured returns `-32062`.

The `task-runner` WIT interface exposes `list-tasks`, `start`, `status`, `poll`, and `cancel`. `start` returns a job ID. Tasks are passive, so `poll` drains queued output, findings, and exit events on the next host call. A plugin may run four tasks at once; starting a fifth returns `-32063`. A task that cannot spawn returns `-32064`. Kawi kills a plugin's tasks when the plugin stops, restarts, reloads, or is disabled.

Task output is line-oriented. Each line is at most 4096 bytes and is truncated on a UTF-8 boundary. A finding path and message are each at most 4096 bytes. A job queue holds 10000 events or 8 MiB, whichever is reached first, and drops the oldest events when full; `dropped-events` counts the drops. Finished jobs remain queryable for 300 seconds.

Tasks run with the workspace root as the default working directory. A task `cwd` may be relative to the workspace root. Task commands are not sandboxed by `process.spawn`; `task.run` is the permission to run the configured project tasks.

## Editor commands

`editor.execute` takes `commands`, an array of built-in editor command IDs. Command IDs are lowercase and may contain dots, hyphens, and underscores. The list must not be empty and wildcards are not supported. Scope entries use canonical IDs such as `file.save` and `edit.format`; aliases are accepted at call time only when the canonical ID is allowed.

The plugin can call `editor.list_commands` to discover the allowed commands and `editor.execute` to run one. Listing returns canonical IDs, descriptions, and aliases. The scope is an allowlist: a command outside the list returns `-32012`. Plugin-defined commands, `keys.send`, and commands that require arguments are not composable and return `-32602`.

The editor runs the command synchronously before responding. Changes made by the command carry the `plugin:<name>` change origin.

## Plugin commands and keymaps

`editor.commands` and `editor.keymaps` enable runtime registration for WebAssembly plugins. Process plugins remain on protocol minor 12 and cannot use them.

`editor.commands` enables the `editor-commands` interface. A plugin registers a command ID, title, description, and aliases during `init` or any invocation. Kawi prefixes the command with the plugin name, so `greet` from plugin `hello` becomes `hello.greet`. Aliases are global, must not collide with an existing command, and obey the same lowercase ID rules. A plugin can register up to 256 commands. IDs are at most 64 bytes, titles 256 bytes, descriptions 1024 bytes, and each command may declare eight aliases.

Runtime commands accept trailing command-line text as one `Text` argument. Manifest commands reject trailing arguments with `E488`, so declare a command in `plugin.toml` when it takes structured arguments.

Registered commands, their aliases, and their key bindings are removed when the plugin stops, restarts, or is disabled. Unregistering a runtime command also removes key bindings that target it.

`editor.keymaps` enables the `editor-keymaps` interface. A plugin sets a binding in normal, insert, or visual mode with a key sequence, a command name, and an optional description. The command must exist. A plugin can register up to 256 bindings. Key sequences are at most 256 bytes and command names 128 bytes.

Kawi rejects a binding owned by the user or by another plugin. A plugin may replace its own binding and may shadow a default binding; removing the plugin binding restores the default. Key bindings are removed with their plugin.

```rust
use kawi_plugin_sdk::{editor_commands, editor_keymaps};

let command = editor_commands::register(&editor_commands::CommandSpec {
    id: "greet".to_string(),
    title: "Greet".to_string(),
    description: Some("Greet the editor".to_string()),
    aliases: vec!["hello".to_string()],
})?;
editor_keymaps::set(&editor_keymaps::KeymapSpec {
    mode: editor_keymaps::KeyMode::Normal,
    keys: "<leader>pg".to_string(),
    command: command.name,
    description: Some("Greet".to_string()),
})?;
```

## Timers and variables

`editor.schedule` enables the `editor-schedule` interface and the `timer.after`, `timer.every`, `timer.cancel`, and `timer.list` host methods. One-shot delays range from 0 ms to 24 hours. Repeating intervals range from 10 ms to 24 hours, and missed ticks coalesce instead of queueing. A payload is at most 4096 bytes, and each plugin can hold 64 timers. Timer firings arrive through `callback-handler.on-callback` as `timer(timer-fired)`. Timers are cleared when the plugin stops, restarts, or is disabled.

`editor.variables` enables the `editor-variables` interface and the `variable.get`, `variable.set`, `variable.delete`, and `variable.list` host methods. Values are booleans, integers, floats, or text. Keys are at most 128 bytes and serialized values at most 4096 bytes. Limits are 1024 global variables, 256 plugin variables, 256 per buffer, and 256 per window. Buffer variables are removed with their buffer and window variables with their window; plugin variables are removed when the plugin stops.

## Option writes

`options.write` enables the `options-write` interface and the `options.set` and `options.reset` host methods. Values are booleans, integers, or text, and the option registry validates the value and scope exactly as it does for `:set`. An unknown option returns `-32074`; an invalid value returns `-32602`. `options.write` does not imply `options.read`, so declare both when the plugin also lists or reads options.

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

## Plugin overlays

`ui.overlay` enables `ui.overlay_open`, `ui.overlay_update`, `ui.overlay_wait`, and `ui.overlay_close`. An overlay is a floating text surface anchored to the cursor, the screen center, or a corner. Only one overlay can be active across all plugins. A second open returns `-32040`; an unknown or foreign overlay returns `-32061`. An active prompt or picker also blocks an overlay open with `-32040`.

Overlays accept at most 1000 lines, 4096 bytes per line, 1 MiB of total line text, a 256-byte title, and a 256-byte status. Width must be between 8 and 160 and defaults to 60. Height must be between 3 and 40 and defaults to the content height plus the border, capped at 12. A value outside either range returns `-32602`. Each span may set an `accent`, `info`, `success`, `warning`, `error`, or `muted` style. Control characters and terminal escapes are removed before display.

`ui.overlay_open` reports the actual width and height. `ui.overlay_update` replaces any supplied field and keeps omitted fields. `ui.overlay_close` removes the overlay and queues a `plugin` close event. The plugin owns selection and scroll state and reports it back through update fields.

A focused overlay receives raw key and paste events through `ui.overlay_wait`. The call suspends the invocation timeout until an event or close arrives. `Esc` and `Ctrl+C` close the overlay with `escape`; `:` closes it with `command` and opens command mode; a click outside closes it with `focus-lost`; `ui.overlay_close` closes it with `plugin`. The `wait` result is always a key, paste, or closed event. Closing queues the closed event, so a plugin may wait after the overlay has already closed.

```rust
use kawi_plugin_sdk::{ui_message, ui_overlay};

let result = ui_overlay::open(&ui_overlay::OverlayOpenParams {
    title: "Quick fix".to_string(),
    status: Some("Enter accepts, Esc closes".to_string()),
    anchor: ui_overlay::OverlayAnchor::Cursor,
    width: Some(60),
    height: Some(8),
    focus: true,
    selected: Some(0),
    scroll: None,
    lines: vec![ui_overlay::OverlayLine {
        spans: vec![ui_overlay::OverlaySpan {
            text: "Replace with the suggested fix".to_string(),
            style: None,
        }],
    }],
})?;

loop {
    match ui_overlay::wait(result.overlay)? {
        ui_overlay::OverlayEvent::Key(event) => {
            if matches!(event.key, ui_overlay::Key::Enter) {
                break;
            }
        }
        ui_overlay::OverlayEvent::Paste(text) => {
            ui_message::message(&format!("pasted {text}"))?;
        }
        ui_overlay::OverlayEvent::Closed(_) => break,
    }
}
```

Overlays are removed when their plugin stops, restarts, reloads, or is disabled.

## Pickers

`ui.picker` enables the blocking `ui.picker` host method and the `ui-picker` WIT interface. A picker shows a native selection list with stable IDs, optional detail text, and a deprecated marker. Kawi provides search, navigation, and cancellation. The call returns `selected` with the item ID or `canceled`. Only one prompt, picker, or overlay can be active at a time; a competing call returns `-32040`.

A picker accepts 1 through 256 items. IDs and detail text are at most 256 and 512 bytes; labels are at most 512 bytes. The initial selection defaults to the first item. The pick suspends the invocation timeout until the user responds.

```rust
use kawi_plugin_sdk::ui_picker;

let outcome = ui_picker::pick(&ui_picker::PickerParams {
    title: "Symbols".to_string(),
    items: vec![ui_picker::PickerItem {
        id: "main".to_string(),
        label: "fn main()".to_string(),
        detail: Some("src/main.rs:1".to_string()),
        deprecated: false,
    }],
    selected: Some(0),
})?;

if let ui_picker::PickerOutcome::Selected(id) = outcome {
    // open the item identified by id
}
```

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
- Runtime commands and key bindings are namespaced to the plugin and removed when it stops. A plugin cannot register a command name or bind keys owned by another plugin.
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
