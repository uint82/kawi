---
title = "Configuration"
aliases = ["config", "settings"]
related = ["options", "keybindings", "themes"]
---
# Configuration

Kawi reads configuration from the XDG config directory, normally `~/.config/kawi/config.toml`.

## Sections

- `[ui]` for keymap, theme, overlays, diagnostics, and completion
- `[editor]` for indentation, line numbers, matching, and autosave
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
```

Options can also be changed at runtime with `:set name=value`, toggled with `:set name!`, queried with `:set name?`, and reset with `:set name&`.

Use `:help options` for the generated option reference.
