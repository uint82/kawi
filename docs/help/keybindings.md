---
title = "Keybindings"
aliases = ["keys", "shortcuts"]
related = ["getting-started", "pickers", "diagnostics"]
---
# Keybindings

The default leader keys are Space and Backslash. You can change them in `keymaps.toml`.

## Leader groups

- `c` code actions and formatting
- `d` diagnostics
- `e` explorer
- `f` files, buffers, replace, and grep
- `r` rename
- `s` symbols
- `t` tasks and terminal
- `v` Git

## Common keys

- `:` command line
- `/` search
- `n` and `N` next and previous match
- `K` hover
- `]d` and `[d` next and previous diagnostic
- `]q` and `[q` next and previous quickfix entry
- `Ctrl-w` window prefix
- `Ctrl-t` return from a help link
- `<leader> e -` open the parent directory of the current file or directory buffer

## Folds

- `za` toggle the fold at the cursor
- `zo` open the fold at the cursor
- `zc` close the fold at the cursor
- `zR` open all folds
- `zM` close all folds
- `zj` and `zk` next and previous fold
- `zd` delete the manual fold at the cursor
- `zf` create a manual fold from a visual selection

## Custom mappings

Create `$XDG_CONFIG_HOME/kawi/keymaps.toml` to bind key sequences to named commands:

```toml
[normal]
"<leader>ff" = "picker.files"
"gx" = "edit.delete_line"
"Q" = { command = "keys.send", args = { keys = "gq" } }

[insert]
"jk" = "mode.normal"
```

Use `:commands` to list command names, `:map` to list configured mappings, and `:map reload` to reload the file. See [Keymaps](help:keymaps) for notation, precedence, timeouts, and limits.

## Key guidance

Pending key sequences show an overlay with available followups. The overlay waits 250 milliseconds by default so fast sequences do not flash.

- `:set keyguide` enables the overlay.
- `:set nokeyguide` disables the overlay without disabling mappings.
- `:set keyguide_delay=0` shows the overlay immediately.

See [Configuration](help:configuration) for details.

## Explorer

The explorer is opt-in and disabled by default. Set `explorer_enabled` to enable it.

- `<leader> e e` toggle the explorer
- `<leader> e f` focus the explorer
- `<leader> e r` reveal the current file
- `:explorer`, `:explorer focus`, and `:explorer reveal` work from the command line

See [Project explorer](help:explorer) for explorer keys, markers, and options.

## Directory buffers

Directory buffers use normal editing keys for the listing.

- `Enter` open the entry under the cursor
- `-` open the parent directory and restore the cursor
- `.` show or hide hidden entries when the buffer is clean
- `:w` apply the edited listing
- `:bd` close the directory buffer and keep Kawi open
- `:bd!` discard pending listing edits and close the directory buffer

See [Directory buffers](help:directory) for the diff rules and safety behavior.

## Pickers

- `<leader> f p` project files
- `<leader> f k` open buffers
- `<leader> f s` live grep
- `<leader> f w` grep current word
- `<leader> f r` search and replace

See [Pickers](help:pickers) and [Diagnostics](help:diagnostics) for more.
