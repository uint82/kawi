---
title = "Writing a WASM plugin"
aliases = ["wasm-plugin"]
related = ["plugins", "plugin-capabilities"]
---
# Writing a WASM plugin

WASM plugins run in a WebAssembly component with a versioned manifest. This guide covers creating, building, publishing, installing, running, and inspecting a Rust plugin. Plugins are experimental and the runtime is optional.

The `kawi-plugin-sdk` crate re-exports the versioned WIT bindings and a small set of helpers. The release workflow publishes version 0.13.0 on crates.io. It pins `wit-bindgen` 0.63.0. Version 0.13.0 bumps the WIT package, so rebuild components built against 0.12.0. Components must implement the exported `callback-handler.on-callback` function. The trim example and conformance suite build with that version and load the component under Wasmtime 50.0.1. A component that targets a different WIT version fails to load with the versioned import name. Other languages work if they target the component model, but only Rust has an SDK and tested template today.

## Prerequisites

Install the WebAssembly target:

```sh
rustup target add wasm32-wasip2
```

Install the component linker required by the WASM build:

```sh
cargo install wasm-component-ld --version 0.5.31 --locked
```

The `wasm` feature is off by default. Enable it when you want to run, inspect, or develop plugins:

```sh
cargo install --path crates/kawi --features wasm
```

Without that feature, `:plugin` commands that need the runtime report that plugin support is unavailable. The component contract is frozen at `kawi:plugin@0.13.0`; do not expect new host interfaces before 1.0.

The scaffold uses the crates.io SDK by default. Pass `--sdk-path` only when developing against an unreleased checkout.

## Scaffold

```sh
kawi plugin new hello-plugin
```

The command writes a crate with:

- `Cargo.toml` using `kawi-plugin-sdk`
- `src/lib.rs` implementing the plugin lifecycle
- `plugin.toml` declaring runtime, component, timeout, activation events, and commands
- `build.sh` wrapping the build and hash step
- `.gitignore` for the local target directory

The generated plugin exposes a `hello` command and declares no host APIs.

## Build

```sh
kawi plugin build hello-plugin
```

The command runs:

```sh
cargo build --release --target wasm32-wasip2
```

It copies the component to the path in `plugin.toml`, computes SHA-256 over the component bytes, and rewrites `sha256`. Do not copy the hash by hand. A hand-copied hash is the most common authoring error.

`build.sh` is a thin wrapper around the same command.

## Publish a release

Managed installation expects a GitHub release. The release commit must contain `plugin.toml` at the repository root. The release must include the component as an asset whose file name matches the manifest `component` file name.

```toml
name = "hello-plugin"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "hello-plugin.wasm"
sha256 = "sha256:..."
```

Attach `hello-plugin.wasm` to the release. Kawi downloads that asset, checks it against `sha256`, and validates the component imports before installing.

A plugin author does not need to publish a registry entry. The repository is the package identity, and the release tag or commit is the version.

## Install

For a released plugin:

```vim
:plugin install owner/repo
:plugin install owner/repo@v0.1.0
```

Kawi resolves the release, reads the manifest, checks the API version, and shows one confirmation with the repository, author, commit, API version, checksum, and declared host APIs. Enter installs and enables the plugin. The commit and checksum are pinned in `$XDG_CONFIG_HOME/kawi/plugins.lock`.

There is no grant step. Installing the plugin is the trust decision. Declared host APIs are enabled automatically.

## Develop locally

Copy the plugin directory into the development plugin directory:

```sh
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins"
cp -r hello-plugin "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins/"
```

Start Kawi. It prints a development plugin warning and marks the plugin as `development` in `:plugin list`.

Inspect the manifest and component imports before running it:

```sh
kawi plugin inspect "${XDG_CONFIG_HOME:-$HOME/.config}/kawi/plugins/hello-plugin"
```

The output lists the declared host APIs and the component imports. Imports must be a subset of the declarations. A component that imports `kawi:plugin/editor-read` must declare `editor.read`, even if it never calls the import.

The default development plugin directory is `$XDG_CONFIG_HOME/kawi/plugins`. Plugin commands are namespaced by plugin name. The scaffold command is:

```vim
:hello-plugin.hello
```

For the bundled WASM trim example, run `:trim.trim` or the configured keybinding.

Development plugins are not managed by `:plugin install`, `:plugin update`, `:plugin disable`, or `:plugin remove`. Edit or remove them directly.

## Host API declarations

- `[capabilities]` in `plugin.toml` declares the host APIs the component uses. A component must declare every interface it imports, except `config-read`, which needs no declaration.
- Kawi enables every declared API for a managed plugin. Declarations are a compatibility and audit surface, not a per-capability grant. Buffer listing and reading use `editor.buffers`; opening files and editing a path buffer use the path-scoped `editor.buffers.write`. Owned scratch buffers use the same capability and stay editable without a path. Window operations use `editor.windows`, cursor and selection control use `editor.control`, option reads use `options.read`, and option changes use `options.write`.
- `capabilities` on a command is an optional invocation gate. It must be a subset of `[capabilities]`.
- `editor.execute` is scoped by an explicit `commands` allowlist. Declare it with `"editor.execute" = { commands = ["edit.format", "file.save"] }`.
- `task.run` is scoped by an explicit `tasks` allowlist. Declare it with `"task.run" = { tasks = ["check", "test"] }`.
- Settings are declared with `[settings.<name>]` tables. The `config-read` interface needs no capability declaration because it only reads the plugin's own namespace.
- `editor.commands` enables runtime command registration; `editor.keymaps` enables runtime key bindings. Both are WebAssembly-only, namespaced to the plugin, and cleared when it stops.
- `editor.decorations` enables highlights, virtual text, and gutter signs. `ui.statusline` enables status line segments.
- A component that imports an undeclared interface is rejected before it runs.
- A denied host call returns `Error` with code `-32012`, a message, and the API name. Handle it by reporting a useful message or stopping the operation.
- On update, Kawi prompts only when the declared API surface grows. The prompt shows the new commit and the added or widened APIs.

## Runtime commands and keymaps

Declare the APIs and register during `init`:

```toml
[capabilities]
"editor.commands" = true
"editor.keymaps" = true
```

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

The returned name is namespaced, so `greet` becomes `<plugin>.greet`. Aliases are global and must not collide. Runtime commands receive trailing command-line text as one `Text` argument. `editor_commands::unregister` removes the command and any bindings that target it. Kawi clears both on stop, restart, and disable.

## Interactive UI

Declare the UI APIs in `plugin.toml`:

```toml
[capabilities]
"ui.prompt" = true
"ui.progress" = true
"ui.panel" = true
"ui.overlay" = true
"ui.picker" = true
```

Prompts block until the user responds and return `PromptOutcome`:

```rust
use kawi_plugin_sdk::types::{Argument, Error, PromptOutcome};
use kawi_plugin_sdk::{ui_message, ui_prompt, INVALID_PARAMS};

let outcome = ui_prompt::input("Name", Some("Ada"), None, Some(64))?;
match outcome {
    PromptOutcome::Text(value) => ui_message::message(&value)?,
    PromptOutcome::Canceled => return Ok(Argument::Null),
    _ => return Err(Error {
        code: INVALID_PARAMS,
        message: "unexpected prompt outcome".to_string(),
        capability: None,
    }),
}
```

`ui_prompt::select` takes a title, a non-empty list of unique options, and an optional initial index. `ui_prompt::confirm` takes a title and an optional message. Each prompt returns `text`, `selected`, `confirmed`, or `canceled`. Only one prompt can be active across all plugins. A conflict returns `UI_BUSY`.

Progress is reported with `ui_progress::start`, `ui_progress::update`, and `ui_progress::finish`:

```rust
use kawi_plugin_sdk::ui_progress;

let id = ui_progress::start("Indexing", Some("src/main.rs"), Some(0.0))?;
for (step, path) in paths.iter().enumerate() {
    if ui_progress::update(id, Some(path.as_str()), Some(step as f32 / paths.len() as f32))? {
        break;
    }
}
ui_progress::finish(id)?;
```

`ui_progress::update` returns `true` after the user cancels. Stop promptly and call `ui_progress::finish`. Only one progress can be active across all plugins. Prompt calls do not count against `timeout_ms`; progress calls do. Users cancel with `:plugin cancel` or a key binding for `plugin.cancel`.

## Panels

Panels render read-only, styled text above the task panel:

```rust
use kawi_plugin_sdk::types::DecorationStyle;
use kawi_plugin_sdk::ui_panel;

let panel = ui_panel::open(&ui_panel::OpenParams {
    title: "Index".to_string(),
    status: Some("building".to_string()),
    height: Some(8),
    focus: true,
    lines: vec![ui_panel::PanelLine {
        text: "src/main.rs".to_string(),
        style: Some(DecorationStyle::Success),
    }],
})?;
ui_panel::update(&ui_panel::UpdateParams {
    panel: panel.panel,
    title: None,
    status: Some("complete".to_string()),
    height: None,
    lines: None,
})?;
ui_panel::close(panel.panel)?;
```

Only one panel can be active across all plugins. A second open returns `UI_BUSY`. Each plugin may update only its own panel. The host owns focus, layout, scrolling, and closing. While focused, `Esc` returns focus to the editor and `q` closes the panel. The panel reserves at least four editor rows and hides when the editor is too small. The `plugin.panel` command toggles focus, and `:plugin panel close` closes it. There is no raw key input API.

## Overlays and pickers

An overlay is a floating text surface for completion menus, quick fixes, and other transient lists. It receives raw key and paste events while focused, so the plugin can implement its own navigation:

```rust
use kawi_plugin_sdk::{ui_message, ui_overlay};

let overlay = ui_overlay::open(&ui_overlay::OverlayOpenParams {
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
    match ui_overlay::wait(overlay.overlay)? {
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

`ui_overlay::update` replaces supplied fields and preserves omitted ones. Use it to move the selection, change the status, or replace the lines. `Esc` and `Ctrl+C` close with `Escape`; `:` closes with `Command` and opens command mode; a click outside closes with `FocusLost`; `ui_overlay::close` closes with `Plugin` and queues a close event. Only one overlay can be active across all plugins; a prompt or picker blocks an overlay open with `UI_BUSY`.

A picker uses the editor's native list and search instead of a custom overlay:

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

match outcome {
    ui_picker::PickerOutcome::Selected(id) => ui_message::message(&format!("selected {id}"))?,
    ui_picker::PickerOutcome::Canceled => {}
}
```

Both calls suspend `timeout_ms` until the user responds. A prompt, picker, and overlay share one interactive slot; a conflict returns `UI_BUSY`.

## Visual customization

Declare the visual APIs and the decoration groups in `plugin.toml`:

```toml
[capabilities]
"editor.decorations" = true
"ui.statusline" = true

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

Group IDs are local to the plugin. They use lowercase letters, digits, hyphens, and underscores, and may not use the built-in style names `accent`, `info`, `success`, `warning`, `error`, or `muted`. `color` selects the semantic base color. `fill` accepts `both`, `foreground`, or `background`. `hidden`, `bold`, `italic`, and `underline` set text attributes. Hidden text is revealed when the cursor is on it.

`editor_decorations::set` replaces every decoration the plugin published for one buffer and revision:

```rust
use kawi_plugin_sdk::editor_decorations;
use kawi_plugin_sdk::types::{Decoration, Highlight, Position, VirtualText};

let decorations = vec![
    Decoration::Highlight(Highlight {
        start: Position { line: 0, byte_column: 0 },
        end: Position { line: 0, byte_column: 4 },
        group: "todo".to_string(),
    }),
    Decoration::VirtualText(VirtualText {
        line: 0,
        text: "indexed".to_string(),
        group: "todo_text".to_string(),
    }),
];
editor_decorations::set(state.buffer, state.revision, &decorations)?;
```

`editor_signs::set` publishes gutter signs. A sign must occupy exactly one terminal cell:

```rust
use kawi_plugin_sdk::editor_signs;

let signs = vec![kawi_plugin_sdk::editor_signs::Sign {
    line: 0,
    text: "!".to_string(),
    group: "todo".to_string(),
}];
editor_signs::set(state.buffer, state.revision, &signs)?;
```

Themes customize a group without changing the plugin:

```toml
[plugin_decorations.markers.todo]
foreground = "#10171a"
background = "#6ed0f0"
fill = "both"
italic = true

[plugin_decorations.markers.todo_text]
foreground = "#6ed0f0"
fill = "foreground"
italic = true
```

Theme overrides apply only to declared groups. Colors may use hex values or palette names. A theme may override colors, fill, and attributes independently.

`ui_statusline::set` replaces the plugin's status line segments. They appear where the `plugins` element is placed in `[ui.statusline]`:

```rust
use kawi_plugin_sdk::ui_statusline;

let segments = vec![ui_statusline::StatuslineSegment {
    text: "indexed".to_string(),
    style: kawi_plugin_sdk::types::DecorationStyle::Success,
    priority: 10,
}];
ui_statusline::set(&segments)?;
```

Decorations are data. The host owns colors, layout, and rendering. They remain until the plugin replaces them, the buffer closes, or the plugin stops, so publish again after a `buffer_changed` event to keep positions current. Diagnostics outrank plugin signs, and plugin signs outrank Git signs. Themes remain separate data packages and are not part of the executable plugin API.

## Autocmds

Declare `editor.autocmds` and implement the exported callback handler. Registration is asynchronous: the callback runs later on the plugin worker.

```rust
use kawi_plugin_sdk::editor_autocmds::{self, AutocmdSpec};
use kawi_plugin_sdk::exports::kawi::plugin::callback_handler::{
    Callback, CallbackEvent, Guest as CallbackGuest,
};
use kawi_plugin_sdk::types::Error;

impl CallbackGuest for Plugin {
    fn on_callback(callback: Callback) -> Result<(), Error> {
        let Callback::Autocmd(callback) = callback else {
            return Ok(());
        };
        match callback.event {
            CallbackEvent::Legacy(event) => handle_event(event),
            CallbackEvent::User(event) => handle_user_event(event),
            _ => {}
        }
        Ok(())
    }
}

fn register_saved() -> Result<(), Error> {
    editor_autocmds::register(&AutocmdSpec {
        event: "file_saved".to_string(),
        pattern: Some("*.rs".to_string()),
        buffer: None,
        once: false,
    })?;
    Ok(())
}
```

An autocmd can register only for events allowed by the plugin's other capabilities. The `user` event needs `editor.autocmds`; buffer and language events need `editor.read`; window events need `editor.windows`. The `window_created`, `window_closed`, `language_changed`, and `user` events are delivered to autocmd callbacks but not to `on_event`.

## Timers

Declare `editor.schedule` to create one-shot and repeating timers. Timers fire asynchronously and arrive through the same `callback-handler.on-callback` export as autocmds.

```rust
use kawi_plugin_sdk::editor_schedule;

editor_schedule::after(250, Some("save"))?;
let timer = editor_schedule::every(5_000, Some("tick"))?;
editor_schedule::cancel(timer)?;
```

`after` accepts a delay from 0 ms to 24 hours. `every` accepts an interval from 10 ms to 24 hours. A payload is at most 4096 bytes. Each plugin can hold 64 timers; repeating timers coalesce missed ticks instead of queueing a backlog.

```rust
use kawi_plugin_sdk::exports::kawi::plugin::callback_handler::{
    Callback, Guest as CallbackGuest,
};
use kawi_plugin_sdk::types::{Error, TimerFired};

impl CallbackGuest for Plugin {
    fn on_callback(callback: Callback) -> Result<(), Error> {
        let Callback::Timer(TimerFired { timer, payload }) = callback else {
            return Ok(());
        };
        handle_timer(timer, payload.unwrap_or_default());
        Ok(())
    }
}
```

Timers are cleared when the plugin stops, restarts, or is disabled. A callback that overruns the plugin timeout is treated like any other callback failure.

## Variables

Declare `editor.variables` to store scalar values in global, plugin, buffer, and window scopes. Values are booleans, 64-bit integers, floats, or text; arrays and objects are rejected.

```rust
use kawi_plugin_sdk::editor_variables::{self, VariableTarget};
use kawi_plugin_sdk::types::{Argument, VariableScope};

let target = VariableTarget {
    scope: VariableScope::Plugin,
    buffer: None,
    window: None,
};
editor_variables::set(target, "indexed", &Argument::Integer(42))?;
let value = editor_variables::get(target, "indexed")?;
let variables = editor_variables::list_variables(target)?;
editor_variables::delete(target, "indexed")?;
```

Buffer and window targets require the corresponding id. Global and plugin targets reject ids. Keys are at most 128 bytes and values at most 4096 bytes when serialized. Limits are 1024 global variables, 256 plugin variables, 256 per buffer, and 256 per window. Buffer variables are removed when the buffer closes and window variables when the window closes; plugin variables are removed when the plugin stops.

## Options

Declare `options.write` to set or reset editor options. The option registry validates the value and scope as it does for `:set`, and the result describes the option after the change.

```rust
use kawi_plugin_sdk::options_write;
use kawi_plugin_sdk::types::Argument;

let info = options_write::set("number", &Argument::Boolean(false))?;
let info = options_write::reset("number")?;
```

`options.set` accepts booleans, integers, and text. Floats, arrays, objects, and null are rejected. Aliases resolve to the canonical option. `options.write` does not imply `options.read`; declare both when the plugin also lists or reads options.

## Project tasks

`task.run` lets a component run tasks configured in the project `tasks.toml` or the global task file. Declare the exact task names:

```toml
[capabilities]
"task.run" = { tasks = ["check", "test"] }
```

The component calls `task_runner::list_tasks`, `start`, `status`, `poll`, and `cancel`. `start` returns a job ID. Tasks run in the background and are passive, so call `poll` to drain output, findings, and the exit event:

```rust
use kawi_plugin_sdk::task_runner::{self, TaskEvent, TaskState};

let _tasks = task_runner::list_tasks()?;
let started = task_runner::start("check")?;
loop {
    let status = task_runner::status(started.job)?;
    if status.state != TaskState::Running {
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(50));
}
for event in task_runner::poll(started.job)? {
    match event {
        TaskEvent::Output(line) => { /* line.channel, line.line */ }
        TaskEvent::Finding(finding) => { /* finding.path, line, column, severity */ }
        TaskEvent::Exited(exited) => { /* exited.exit_code, success, cancelled */ }
    }
}
```

A task outside the declared scope returns `-32012`. A declared name that is not configured returns `-32062`. Four tasks may run at once; a fifth returns `-32063`. A failed spawn returns `-32064`. Output lines are capped at 4096 bytes, queues drop the oldest events, and finished jobs remain queryable for 300 seconds.

## Settings

Declare settings in `plugin.toml`:

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

Supported types are `boolean`, `integer`, `number`, and `string`. `enum` is available for strings. `minimum` and `maximum` are available for integers and numbers. Defaults must match the type and satisfy the constraints.

Users override settings in `$XDG_CONFIG_HOME/kawi/plugins.toml`:

```toml
[hello-plugin]
max_items = 25
mode = "dark"
```

Users can edit them through `:plugin settings hello-plugin`. The editor shows the declared type, default, choices, and range, validates each change, and preserves comments and unrelated tables in `plugins.toml`. Boolean settings toggle on Enter, enumerated strings open a choice picker, and other values use a text prompt. Ctrl+R removes the user override and restores the default.

Read them through the SDK:

```rust
use kawi_plugin_sdk::{config_read, types::SettingValue};

let max_items = match config_read::get("max_items")? {
    Some(SettingValue::Integer(value)) => value,
    _ => 10,
};
let all = config_read::get_all()?;
```

`config_read::get` returns `None` for an unknown key. `config_read::get_all` returns settings sorted by key.

## Contract details

- Positions use zero-based lines and UTF-8 byte columns, not character columns. A multibyte codepoint advances the byte column by its encoded length. Use `kawi_plugin_sdk::position_at` and `byte_offset` instead of counting characters.
- `editor.get_state` returns a revision. Pass that revision to `editor.apply_edit`. A stale revision returns code `-32010`.
- `kawi_plugin_sdk::apply_edit_with_retry` fetches fresh state, rebuilds edits, and retries a bounded number of times. Declare `editor.read` and `editor.write` to use it.
- `editor_execute::list_commands` returns only commands in the allowlist. `editor_execute::execute` runs a canonical built-in command. Plugin-defined commands, `keys.send`, and commands that require arguments are rejected. The editor runs the command before the host call returns.
- `config_read::get` returns one effective setting value as `option<setting-value>`. `config_read::get_all` returns every effective setting. Values come from the manifest defaults overlaid with the user's `plugins.toml`. Invalid user values fall back to their defaults and are logged. Kawi reads the file on demand, so changes apply on the next call without restarting the plugin.
- `buffer_changed` includes an origin. Kawi excludes your own edits, so do not filter by origin to avoid loops. A `mixed` origin includes your own edit or changes from several sources; resync when `full_resync` is true.
- The editor coalesces and orders events. A `file_saved` event is a good trigger for a full-buffer operation such as trim.
- Diagnostics accept at most 1000 items per buffer. Each message is at most 4096 bytes and each code is at most 256 bytes.
- The invocation budget defaults to 5 seconds and can be set from 100 ms to 60 seconds with `timeout_ms`. A timeout traps the call and counts toward restart backoff.
- WebAssembly threads are not enabled. Do not start threads or rely on shared memory.
- There is no ambient network access. `net.connect` is deny by default and WASI sockets are the only network surface.
- Network limits are 64 simultaneous TCP connections, 8 MiB inbound and 8 MiB outbound per connection, and 64 MiB shared per session. A session that exceeds a cap gets a short read or a closed write, not an unbounded allocation.
- WASI filesystem access is scoped by declared `fs.read` and `fs.write` paths. The component can only see preopened paths inside those scopes. Preopens are directories; a file path is skipped with a log line.

## Real edit example

`examples/wasm-trim` is the reference for a real edit. It reads state with `editor.get_state`, computes zero-based line and UTF-8 byte-column edits, and calls `apply_edit_with_retry`. The helper retries a stale revision twice, refuses to continue when the buffer identity changes, and returns `STALE_REVISION` after the retry budget. Report that error instead of applying the edits to a new buffer.

## Troubleshooting

### The component imports an API that is not declared

The runtime refuses the component with a message naming the unauthorized import. Add the API to `[capabilities]`, rebuild, and publish a new release.

### The component hash does not match

Run `kawi plugin build <dir>` again. Do not edit `sha256` manually. For a managed plugin, publish a new release and run `:plugin update`. For a development plugin, run `:plugin restart <name>`.

### The plugin traps or stays in backoff

Open `:plugin logs <name>`. A trap is reported with the WebAssembly error. Repeated traps increase restart backoff. Fix the trap rather than lowering the timeout.

### A host call is denied

A denied call returns `-32012`. The component tried to use an interface it did not declare or a host method outside the declared API. Add the declaration and publish a new release, or remove the call.

## Honest scope

The Rust SDK and template are the supported authoring path. Any language that can produce a component targeting `kawi:plugin@0.13.0` can work, but it must provide its own bindings, UTF-8 position handling, stale-revision retry, and error mapping. The conformance suite is the source of truth for behavior across runtimes.
