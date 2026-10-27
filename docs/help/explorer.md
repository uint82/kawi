---
title = "Project explorer"
aliases = ["file-tree", "tree"]
related = ["keybindings", "configuration", "sessions"]
---
# Project explorer

The project explorer shows the repository tree next to the editor. It marks Git changes and diagnostics, highlights the active buffer, and supports file management without leaving the editor.

Opening the explorer roots the tree at the current file's project and reveals the file, expanding its parent directories. Without an open file, the tree uses the working directory.

The explorer closes automatically when the terminal cannot leave its minimum width for the editor.

## Commands

- `<leader> e e` toggle the explorer
- `<leader> e f` focus the explorer
- `<leader> e r` reveal the current file in the explorer
- `<leader>` shortcuts remain available while the explorer is focused
- `:explorer` toggle the explorer
- `:explorer focus` focus the explorer
- `:explorer reveal` reveal the current file

## Explorer keys

- `j` and `k` move down and up
- `g` and `G` jump to the first and last item
- `h` collapse the selected directory or move to the parent
- `l` expand the selected directory or open the selected file
- `Enter` toggle the selected directory or open the selected file
- `o` open in the current window
- `v` open in a vertical split
- `s` open in a horizontal split
- `a` create a file
- `A` create a directory
- `r` rename the selected path
- `d` delete the selected path
- `R` refresh the tree
- `.` show or hide dotfiles
- `q` closes the explorer
- `Esc` or `Ctrl-C` return focus to the editor

Double click a row to toggle a directory or open a file. The mouse wheel scrolls the tree.

## Markers

- `*` marks a buffer with unsaved changes
- `E`, `W`, `I`, and `H` mark the highest diagnostic severity in a file or directory
- Git status letters follow the Git integration markers, with conflicts shown as `!`

Hidden dotfiles are filtered by default. Version control directories are always hidden.

## Configuration

```toml
[ui]
explorer_width = 30
explorer_show_hidden = false
```

- `explorer_width` sets the panel width from 20 to 60 columns.
- `explorer_show_hidden` includes hidden files and directories.

Use `:set explorer_width=40` and `:set explorer_show_hidden` at runtime. The explorer tree, width, visibility, and selection are restored with the workspace session.

See [Keybindings](help:keybindings) and [Sessions](help:sessions) for related behavior.
