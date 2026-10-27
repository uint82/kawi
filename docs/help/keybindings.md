---
title = "Keybindings"
aliases = ["keys", "shortcuts"]
related = ["getting-started", "pickers", "diagnostics"]
---
# Keybindings

The leader key is Space or Backslash.

## Leader groups

- `c` code actions and formatting
- `d` diagnostics
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

## Folds

- `za` toggle the fold at the cursor
- `zo` open the fold at the cursor
- `zc` close the fold at the cursor
- `zR` open all folds
- `zM` close all folds
- `zj` and `zk` next and previous fold
- `zd` delete the manual fold at the cursor
- `zf` create a manual fold from a visual selection

## Key guidance

Pending key sequences show an overlay with available followups. The overlay waits 150 milliseconds by default so fast sequences do not flash.

- `:set keyguide` enables the overlay.
- `:set nokeyguide` disables the overlay without disabling mappings.
- `:set keyguide_delay=0` shows the overlay immediately.

See [Configuration](help:configuration) for details.

## Pickers

- `<leader> f p` project files
- `<leader> f k` open buffers
- `<leader> f s` live grep
- `<leader> f w` grep current word
- `<leader> f r` search and replace

See [Pickers](help:pickers) and [Diagnostics](help:diagnostics) for more.
