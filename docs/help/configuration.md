---
title = "Configuration"
aliases = ["config", "settings"]
related = ["options", "keybindings", "themes"]
---
# Configuration

Kawi reads configuration from the XDG config directory, normally `~/.config/kawi/config.toml`.

## Sections

- `[ui]` for keymap, theme, overlays, diagnostics, and completion
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
