---
title = "Git"
aliases = ["version-control"]
related = ["configuration", "keybindings"]
---
# Git

Kawi reads Git state for the active workspace and provides status, staging, commits, branches, and diffs.

## Commands

- `:git status`
- `:git branches`
- `:git log`
- `:git switch <branch>`
- `:git commit`
- `:git commit all`
- `:git push`
- `:git fetch`
- `:git next_hunk`
- `:git prev_hunk`
- `:git diff`

Use `:help options` for related configuration.

## Keys

- `<leader> v d` show diff
- `<leader> v p` preview hunk
- `<leader> v s` stage hunk
- `<leader> v S` stage buffer
- `<leader> v u` unstage hunk
