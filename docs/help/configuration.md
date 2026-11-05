---
title = "Configuration"
aliases = ["config", "settings"]
related = ["options", "keybindings", "themes", "debug"]
---
# Configuration

Kawi reads configuration from the XDG config directory, normally `~/.config/kawi/config.toml`. Custom key mappings live in a separate `keymaps.toml` file.

## Sections

- `[ui]` for keymap, theme, overlays, diagnostics, completion, the project explorer, and the status line
- `[editor]` for indentation, line numbers, sign column, matching, autosave, and key guidance
- `[explorer]` for directory buffer behavior
- `[history]` for undo history and persistence

Example:

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
```

Options can also be changed at runtime with `:set name=value`, toggled with `:set name!`, queried with `:set name?`, and reset with `:set name&`.

## Indentation

Kawi applies per-language indentation defaults. Rust, Python, C, C++, Java, C#, and PHP use 4 spaces. TypeScript, JavaScript, JSON, HTML, CSS, YAML, and Ruby use 2 spaces. Go uses tabs. The built-in defaults follow the formatter and style guide for each language.

Indentation settings resolve in this order, from highest priority to lowest:

1. `:set` in the current buffer
2. A matching `[[language]]` entry in `languages.toml`
3. Explicit `[editor]` settings in `config.toml`
4. Built-in language defaults
5. Built-in fallback

Add per-language overrides in `$XDG_CONFIG_HOME/kawi/languages.toml`:

```toml
[[language]]
name = "rust"
shiftwidth = 2
expandtab = true
tabstop = 2
```

`shiftwidth` sets the indentation unit. `expandtab` inserts spaces instead of tabs. `tabstop` sets the display width of a tab.

## Keymaps

Custom mappings live in `$XDG_CONFIG_HOME/kawi/keymaps.toml`, normally `~/.config/kawi/keymaps.toml`. They are kept separate from options so plugins and users can share mappings without rewriting `config.toml`.

```toml
leader = [" ", "\\"]
timeout = 1000

[normal]
"<leader>ff" = "picker.files"

[insert]
"jk" = "mode.normal"
```

Use `:commands` to discover command names, `:map` to inspect mappings, and `:map reload` to reload the file. See [Keymaps](help:keymaps) for the complete reference.

## Debugging

Debug adapters and launch configurations live in `$XDG_CONFIG_HOME/kawi/debug.toml`, normally `~/.config/kawi/debug.toml`. Projects can override them with `.kawi/debug.toml`. See [Debugging](help:debug) for the configuration format and commands.

## Plugins

Managed plugins are installed from GitHub with `:plugin install owner/repo`. Installed code lives in `$XDG_DATA_HOME/kawi/plugins`, normally `~/.local/share/kawi/plugins`. The lockfile lives in `$XDG_CONFIG_HOME/kawi/plugins.lock`, normally `~/.config/kawi/plugins.lock`.

Development plugins live in `$XDG_CONFIG_HOME/kawi/plugins`. They are loaded with a warning and are not managed by `:plugin`.

Plugin settings live in `$XDG_CONFIG_HOME/kawi/plugins.toml`, normally `~/.config/kawi/plugins.toml`, under the plugin name:

```toml
[echo]
max_items = 25
mode = "dark"
```

Each plugin declares the supported keys, types, defaults, and ranges in its manifest. See [Plugins](help:plugins) and [Plugin host APIs](help:plugin-capabilities).

Directory buffer settings:

```toml
[explorer]
show_hidden = false
confirm = true
trash = true
sort = "name"
sort_reverse = false
sort_directories_first = true
metadata = ["permissions", "links", "owner", "group", "size", "modified"]
```

See [Directory buffers](help:directory) for the behavior of these options.

Use `:help options` for the generated option reference. See [Themes](help:themes) for built-in and custom themes.

## Discovery

Press Tab in command mode to complete commands, option names, option values, and help topics.

- `:set <Tab>` lists option names.
- `:set theme <Tab>` lists built-in and custom themes.
- `:set signcolumn <Tab>` lists the accepted values.
- `:set no<Tab>` lists boolean options that can be negated.
- `:help <Tab>` lists help topics.
- `:help option <Tab>` lists options.
- `:help option <name>` shows one option, including its description and values.

## Key guidance

Kawi shows a transient overlay for pending key sequences. The overlay lists the available followups and updates after each key.

- `keyguide` controls the overlay and defaults to `true`.
- `keyguide_delay` sets the delay in milliseconds and defaults to `250`. Set it to `0` for immediate display.
- The overlay covers leader menus and Vim sequences such as `dd`, `gg`, `ciw`, `g~`, `z`, and `[`.
- Dynamic prefixes show a hint instead of a list, such as a target character for `f` or a register for `"`.
- Disabling the overlay does not disable the leader key or any key mappings.

```toml
[editor]
keyguide = true
keyguide_delay = 250
```


## Folding

Folds collapse ranges of lines into a single visible line. Set `foldmethod` to `manual` or `indent`.

- `manual` uses folds created with `zf` in visual mode.
- `indent` derives folds from indentation and the effective `shiftwidth`.
- `foldcolumn` controls the fold marker column width. Set it to `0` to hide the column. The column is reserved even when no fold is present, so the layout does not shift when folds appear.
- Folds are saved per file in the state directory and restore regardless of how Kawi starts. Manual folds restore when the file content matches. Closed indentation folds restore when their fold starts still exist.

```toml
[editor]
foldmethod = "indent"
foldcolumn = 1
```

Use `za` to toggle a fold, `zo` and `zc` to open and close, `zR` and `zM` to open or close all folds, and `zj` and `zk` to move between fold starts. Use `zf` in visual mode to create a manual fold and `zd` to delete one.

## Sign column

The sign column sits left of the line numbers and displays diagnostics and Git changes.

`signcolumn` accepts `never`, `auto`, or `always`. It defaults to `never`.

- `never` hides the sign column.
- `auto` shows the sign column when the buffer has signs.
- `always` reserves the sign column even when it is empty.

`git_signs` controls Git signs and defaults to `false`. Its alias is `git_sign`. Diagnostic signs are shown whenever the sign column is visible.

To show Git changes while keeping the column compact:

```toml
[editor]
signcolumn = "auto"
git_signs = true
```

## Status line

The status line is built from element lists for the left, center, and right sections. Elements render only when they have content, and empty elements do not leave separators. The `plugins` element expands to the segments published by running plugins and is hidden in inactive windows.

```toml
[ui.statusline]
left = ["mode", "file", "modified", "readonly", "git"]
center = []
right = ["plugins", "filetype", "encoding", "position"]
separator = " "
color_modes = false

[ui.statusline.mode]
normal = "-- NORMAL --"
insert = "-- INSERT --"
replace = "-- REPLACE --"
visual = "-- VISUAL --"
command = "-- COMMAND --"
search = "-- SEARCH --"
```

Available elements:

- `mode`, `file`, `modified`, `readonly`
- `diagnostics`, `workspace_diagnostics`
- `git`, `lsp`
- `selection`, `macro`
- `filetype`, `indent`, `encoding`, `eol`
- `percent`, `position`, `plugins`

The `file` element is flexible and truncates from the left when space is short, keeping the end of the path. `mode` and `position` are the last elements to survive. Inactive windows show only the file and modified state.

`diagnostics` follows `diagnostic_counts`. `workspace_diagnostics` is opt-in and follows `diagnostic_workspace_counts`. `position` shows the line and character column, adding the display column when it differs. `encoding` and `eol` render only when the buffer uses a non-default value. `percent` is the cursor position through the file.

`color_modes` defaults to `false`. Set it to `true` to render colored segments with a background badge. The separator is sanitized before rendering, so control characters and escape sequences never reach the terminal.
