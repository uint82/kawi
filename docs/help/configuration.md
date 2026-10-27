---
title = "Configuration"
aliases = ["config", "settings"]
related = ["options", "keybindings", "themes"]
---
# Configuration

Kawi reads configuration from the XDG config directory, normally `~/.config/kawi/config.toml`.

## Sections

- `[ui]` for keymap, theme, overlays, diagnostics, completion, and the status line
- `[editor]` for indentation, line numbers, sign column, matching, and autosave
- `[history]` for undo history and persistence

Example:

```toml
[ui]
keymap = "vim"
theme = "default"
diagnostic_virtual_text = "all"

[editor]
shiftwidth = 4
expandtab = true
number = true
signcolumn = "auto"
git_signs = true
```

Options can also be changed at runtime with `:set name=value`, toggled with `:set name!`, queried with `:set name?`, and reset with `:set name&`.

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

The status line is built from element lists for the left, center, and right sections. Elements render only when they have content, and empty elements do not leave separators.

```toml
[ui.statusline]
left = ["mode", "file", "modified", "readonly", "diagnostics", "git", "lsp"]
center = []
right = ["selection", "macro", "filetype", "indent", "encoding", "eol", "percent", "position"]
separator = " "
color_modes = true

[ui.statusline.mode]
normal = "NORMAL"
insert = "INSERT"
replace = "REPLACE"
visual = "VISUAL"
command = "COMMAND"
search = "SEARCH"
```

Available elements:

- `mode`, `file`, `modified`, `readonly`
- `diagnostics`, `workspace_diagnostics`
- `git`, `lsp`
- `selection`, `macro`
- `filetype`, `indent`, `encoding`, `eol`
- `percent`, `position`

The `file` element is flexible and truncates from the left when space is short, keeping the end of the path. `mode` and `position` are the last elements to survive. Inactive windows show only the file and modified state.

`diagnostics` follows `diagnostic_counts`. `workspace_diagnostics` is opt-in and follows `diagnostic_workspace_counts`. `position` shows the line and character column, adding the display column when it differs. `encoding` and `eol` render only when the buffer uses a non-default value. `percent` is the cursor position through the file.

Set `color_modes = false` to render the mode badge without its background color. The separator is sanitized before rendering, so control characters and escape sequences never reach the terminal.
