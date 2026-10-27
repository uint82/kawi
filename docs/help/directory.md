---
title = "Directory buffers"
aliases = ["oil", "dired", "file manager"]
related = ["explorer", "keybindings", "configuration", "sessions"]
---
# Directory buffers

A directory buffer lists a directory as editable text. The buffer is an intent: editing lines does not touch the filesystem until you write. This follows the oil.nvim and wdired model.

Open a directory with `kawi .`, `:e dir/`, or `-` in a file or directory buffer. `-` opens the parent directory and places the cursor on the entry you came from.

## Editing

Each line is a name. Directories end with `/`.

- Change a name and write to rename an entry.
- Remove a line and write to delete an entry.
- Add a line and write to create an entry.
- Duplicate a line, rename the copy, and write to copy an entry.
- Use a relative path such as `sub/new.txt` to create parent directories.
- Move an entry by writing a relative path such as `sub/existing.txt`.

Names are stored as `OsString` values. Unchanged lines are never renamed, so names that cannot be represented in UTF-8 remain untouched.

`Enter` opens the entry under the cursor. `.` toggles hidden entries when the buffer has no unsaved changes.

The buffer is a normal modified buffer. `:q` with pending changes is refused like any other buffer. After a successful write the listing reloads from disk and the text undo history is cleared.

## Safety

Before writing, Kawi validates names, duplicate targets, missing parents, existing targets, and directory moves into themselves. Rename cycles and swaps use temporary names.

Deletes go to the freedesktop trash by default. Unlike symlinks, the link itself is renamed or removed and never followed. Copies preserve symlinks and metadata, and cross-device moves fall back to copy then delete.

Plans with deletes or multiple changes ask for confirmation. A single rename or create applies immediately. Execution stops at the first filesystem error, reports what was applied, and reloads the listing so the buffer matches disk.

The directory is checked again before applying. If entries changed since the buffer loaded, Kawi refuses the write and asks you to reload.

## Commands and keys

- `:e dir/` open a directory buffer
- `kawi .` start in the current directory
- `-` open the parent directory
- `Enter` open the entry under the cursor
- `.` toggle hidden entries when the buffer is clean
- `:w` apply pending changes
- `:w!` apply without the directory confirmation prompt

Open buffers whose paths are renamed are retargeted, buffers under deleted paths are closed, and LSP rename notifications are sent after a successful apply.

## Configuration

```toml
[explorer]
show_hidden = false
confirm = true
trash = true
sort = "directories"
```

- `show_hidden` includes dotfiles in new directory buffers.
- `confirm` asks before deletes and multi-change plans.
- `trash` sends deletes to the trash. Set it to `false` for permanent deletes.
- `sort` is `directories` for directories first, or `name` for natural name order.

The runtime options are `directory_show_hidden`, `directory_confirm`, `directory_trash`, and `directory_sort`.

Directory buffers restore in workspace sessions by path. Unsaved listing edits are not persisted across sessions.

See [Project explorer](help:explorer), [Keybindings](help:keybindings), and [Sessions](help:sessions) for related behavior.
