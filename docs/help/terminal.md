---
title = "Terminal"
aliases = ["term", "shell"]
related = ["tasks", "keybindings"]
---
# Terminal

Kawi provides horizontal and vertical terminal panels.

## Commands

- `:terminal` open a shell
- `:terminal <command>` run a command
- `:vterminal` open a vertical shell
- `:vterminal <command>` run a command vertically

Toggle the terminal with `<leader> t t` or the optional configured focus key.

Terminal panels keep their output while you continue editing. Use the task commands to cancel or close running work.

## Focus

The terminal focus key is unbound by default. Set `terminal_focus_key` to a key such as `ctrl-g` to enable one-key toggling in both directions. The key opens the terminal when it is closed and takes focus from the project explorer, debug panel, or plugin panel. The panel stays open when focus returns to the editor.

Set `terminal_focus_key = "none"` to disable it again. Change the key at runtime or in the editor configuration:

```toml
[editor]
terminal_focus_key = "ctrl-g"
```

Inside the terminal, `Ctrl-\` then `Ctrl-n` enters terminal normal mode. `Esc` returns focus to the editor, `q` closes the panel, and `j`, `k`, the arrow keys, and the mouse wheel scroll. In terminal normal mode, `Ctrl-w h`, `Ctrl-w j`, `Ctrl-w k`, and `Ctrl-w l` move focus to another pane or panel.
