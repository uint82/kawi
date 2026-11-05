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
- `:git next_conflict`
- `:git prev_conflict`
- `:git ours`
- `:git theirs`
- `:git both`
- `:git continue`
- `:git abort`

Use `:help options` for related configuration.

## Keys

- `<leader> v d` show diff
- `<leader> v p` preview hunk
- `<leader> v s` stage hunk
- `<leader> v S` stage buffer
- `<leader> v u` unstage hunk
- `<leader> v n` next conflict
- `<leader> v N` previous conflict
- `<leader> v o` take ours for the conflict at the cursor
- `<leader> v t` take theirs for the conflict at the cursor
- `<leader> v b` take both sides for the conflict at the cursor

## Conflicts

Conflict markers show a sign in the gutter. Staging and committing are blocked while a buffer or the index still contains markers. Edit the block manually or use `ours`, `theirs`, or `both` inside a conflict block.

`:git continue` completes an in-progress merge, rebase, cherry-pick, or revert. `:git abort` cancels it after confirmation.
