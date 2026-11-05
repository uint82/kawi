<h1 align="center">Kawi</h1>

<p align="center">
  A fast, modal terminal editor with Vim keymaps, language server support, fuzzy pickers, tasks, terminals, and Git.
</p>

<p align="center">
  <a href="https://github.com/uint82/kawi/actions/workflows/ci.yml">
    <img src="https://github.com/uint82/kawi/actions/workflows/ci.yml/badge.svg" alt="CI status">
  </a>
</p>

> Kawi is in early development. The Vim keymap is usable today. Commands, options, and configuration may change.

## Overview

Kawi is built for editing in the terminal without giving up modern editor features. It combines modal editing with project-wide search, language tooling, integrated terminals, task execution, and Git workflows.

## Features

- **Modal editing.** Vim keymap with multiple windows and splits.
- **Language servers.** Completion, hover, diagnostics, symbols, rename, formatting, code actions, and start, stop, or restart controls.
- **External formatters.** Configure formatter commands in `languages.toml`, format with `:format`, and enable format-on-save.
- **Syntax highlighting.** Tree-sitter parsing for Rust, Python, JavaScript, TypeScript, JSX, TSX, JSON, TOML, Bash, C, C++, Go, Java, C#, Ruby, PHP, Lua, YAML, HTML, CSS, and Markdown.
- **Fuzzy search.** Pickers for files, buffers, live grep, replace, and symbols, with previews where supported.
- **Project explorer.** Opt-in file tree with file management, Git markers, diagnostics, and session restore.
- **Directory buffers.** Edit a directory listing and apply renames, copies, creates, and trash deletes on write.
- **Diagnostics.** Project-wide diagnostics, quickfix navigation, and configurable display.
- **Git integration.** Status, staging, commits, branches, hunk navigation, and optional gutter signs.
- **Tasks and terminals.** Project task runner, task output, cancellation, and horizontal or vertical terminal panels.
- **Sessions and recovery.** Workspace restore, crash recovery, autosave, macros, undo history, and persistent undo.
- **Folding.** Manual folds, indentation folds, fold navigation, a configurable fold column, and per-file persistence.
- **Custom keymaps.** Bind named commands in `keymaps.toml`, configure the leader and ambiguity timeout, and inspect mappings with `:map`.
- **Development plugins.** Local process and WASM plugins for development, loaded with a warning and optional Linux Landlock filesystem and process-execution scopes.
- **Managed WASM plugins.** Install GitHub releases with checksum verification, a pinned lockfile, explicit updates, and a versioned component contract. Includes a published Rust SDK, build tooling, authoring guide, and conformance-backed trim example. Build with `--no-default-features` to omit the runtime.
- **Discoverable keys.** Leader menus and key sequence guidance for pending Vim commands.
- **Discoverable configuration.** Tab completion, generated option metadata, and built-in help pages.
- **Themes.** Built-in light and dark themes with support for custom themes.

## Requirements

- Rust 1.98 or newer.
- Git.
- A Unix-like operating system. CI currently runs on Linux.

## Installation

### Build from source

```sh
git clone https://github.com/uint82/kawi.git
cd kawi
cargo build --release --locked
./target/release/kawi
```

### Install with Cargo

```sh
cargo install --path crates/kawi --locked
```

## Quick start

```sh
kawi
kawi src/main.rs
kawi doctor
```

Kawi opens an empty workspace when no file is provided and restores the previous workspace when possible. Use `kawi doctor` to validate configuration, language servers, tasks, Git, sessions, and state directories.

Press Tab in command mode to complete commands, options, option values, and help topics. The in-editor guide is available with `:help`, `:help options`, and `:help commands`.

## Key bindings

Kawi currently uses a Vim-style keymap.

| Key | Action |
| --- | --- |
| `i` | Enter insert mode |
| `Esc` | Return to normal mode |
| `:` | Open the command line |
| `/` | Search |
| `K` | Show hover information |
| `<Space> e e` | Toggle the project explorer |
| `<Space> f p` | Find project files |
| `<Space> f s` | Live grep |
| `]d` and `[d` | Next and previous diagnostic |
| `Ctrl-w` | Window prefix |
| `Ctrl-t` | Return from a help link |

The default leader keys are Space and Backslash. Change them or add your own mappings in `$XDG_CONFIG_HOME/kawi/keymaps.toml`. See the [keybindings guide](docs/help/keybindings.md) and the [keymap guide](docs/help/keymaps.md) for details.

## Configuration

Kawi reads configuration from `$XDG_CONFIG_HOME/kawi/config.toml`, normally `~/.config/kawi/config.toml`.

```toml
[ui]
keymap = "vim"
theme = "default"
diagnostic_virtual_text = "all"
diagnostic_underline = "curly"

[editor]
shiftwidth = 4
expandtab = true
number = true
signcolumn = "auto"
git_signs = true
foldmethod = "manual"
foldcolumn = 1
autosave = "off"
keyguide = true
keyguide_delay = 250
```

Options can be changed at runtime with `:set name=value`, toggled with `:set name!`, queried with `:set name?`, and reset with `:set name&`.

Language servers are configured in `$XDG_CONFIG_HOME/kawi/languages.toml`. Debug adapters are configured in `$XDG_CONFIG_HOME/kawi/debug.toml`. Project tasks are configured in `$XDG_CONFIG_HOME/kawi/tasks.toml`. Custom key mappings live in `$XDG_CONFIG_HOME/kawi/keymaps.toml`. Managed plugins are installed with `:plugin install owner/repo`. Development plugins live in `$XDG_CONFIG_HOME/kawi/plugins`. Custom themes and theme packages belong in `$XDG_CONFIG_HOME/kawi/themes`; themes are data packages, not plugins.

See the [configuration guide](docs/help/configuration.md) and [theme guide](docs/help/themes.md) for details.

## Documentation

- [Getting started](docs/help/getting-started.md)
- [Features](docs/help/features.md)
- [Keybindings](docs/help/keybindings.md)
- [Keymaps](docs/help/keymaps.md)
- [Plugins](docs/help/plugins.md)
- [Plugin host APIs](docs/help/plugin-capabilities.md)
- [Writing a WASM plugin](docs/help/plugin-authoring.md)
- [Migrating a process plugin to WASM](docs/help/plugin-migration.md)
- [Configuration](docs/help/configuration.md)
- [Language servers](docs/help/lsp.md)
- [Debugging](docs/help/debug.md)
- [Formatting](docs/help/formatting.md)
- [Troubleshooting](docs/help/troubleshooting.md)

Use `:help <topic>` to open any topic inside Kawi.

## Development

The workspace is split into focused crates:

| Crate | Responsibility |
| --- | --- |
| `kawi` | Application shell, configuration, sessions, and integration |
| `kawi-editor` | Editing model, commands, motions, modes, and completion |
| `kawi-buffer` | Buffers, undo, recovery storage, and file writes |
| `kawi-tui` | Terminal rendering, themes, frames, and overlays |
| `kawi-lsp` | Language server lifecycle and protocol handling |
| `kawi-dap` | Debug adapter lifecycle and protocol handling |
| `kawi-syntax` | Tree-sitter parsing and syntax highlighting |
| `kawi-picker` | Fuzzy finding, filtering, and previews |
| `kawi-git` | Repository status, diffs, staging, commits, and branches |
| `kawi-task` | Task configuration, execution, and output parsing |
| `kawi-terminal` | PTY sessions, terminal state, and input encoding |
| `kawi-help` | Embedded help pages and topic index |
| `kawi-completion`, `kawi-process`, `kawi-watch` | Supporting services |

Run the editor from the workspace with `cargo run -p kawi -- path/to/file`.

Before submitting changes, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Contributing

Bug reports and pull requests are welcome. Keep changes focused, add tests for behavior changes, and make sure the full check suite passes.

## License

Kawi is licensed under the GNU General Public License, version 3 or later (`GPL-3.0-or-later`). See [LICENSE](LICENSE) for the full license text.
