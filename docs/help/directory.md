---
title = "Directory buffers"
aliases = ["oil", "dired", "file manager"]
related = ["explorer", "keybindings", "configuration", "sessions"]
---
# Directory buffers

A directory buffer lists a directory as editable text. The buffer is an intent: editing lines does not touch the filesystem until you write. This follows the oil.nvim and wdired model.

Listings load in the background. The buffer is read-only until its listing arrives, so opening a large directory does not block navigation or other buffers. Writes recheck the directory in the background before applying changes.

Open a directory with `kawi .`, `:e dir/`, or `-` in a directory buffer. From a file buffer, `<leader> e -` opens the parent directory and places the cursor on the file you came from. `-` keeps its normal Vim motion in file buffers.

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

The buffer is a normal modified buffer. `:q` closes the window, so with one window it exits Kawi. Use `:bd` to close only the directory buffer. `:q` and `:bd` refuse pending listing edits without `!`. After a successful write the listing reloads from disk and the text undo history is cleared.

## Metadata

Directory metadata is display-only text before each entry name, in permissions, links, owner, group, size, and modified order. It is not part of the buffer, so it does not affect editing, search, or writes.

```toml
[explorer]
metadata = ["permissions", "links", "owner", "group", "size", "modified"]
```

- `permissions` shows Unix mode bits. Other platforms show unknown permissions.
- `links` shows the hard link count. Other platforms show `?`.
- `owner` shows the owner name resolved from the user database, falling back to the numeric ID. Other platforms show `?`.
- `group` shows the group name resolved from the group database, falling back to the numeric ID. Other platforms show `?`.
- `size` is human readable using powers of 1024. Directories show `-`.
- `modified` shows local time in `ls` style, such as `Nov  1 20:26` for recent entries and `Nov  1  2025` for older entries.

All six columns are shown by default. Set `:set directory_metadata=none` or use `metadata = []` to hide metadata. Use a comma-separated list at runtime, such as `:set directory_metadata=permissions,size`. Owner and group column widths grow to fit the widest value in the listing.

Metadata is read with the listing on the background worker. Changing the option affects the next load, so use `:e` to refresh an open directory buffer. Rows without loaded metadata keep their column space blank.

## Safety

Before writing, Kawi validates names, duplicate targets, missing parents, existing targets, directory moves into themselves, moves into a directory that is being renamed, and permissions for affected directories, copy sources, and deletes. Permission checks cover search access on parents, write access on directories moved across parents, sticky-directory ownership, and the trash for the target filesystem. All permission failures are listed together, naming each operation and path. Rename cycles and swaps use temporary names.

Deletes go to the freedesktop trash by default. Unlike symlinks, the link itself is renamed or removed and never followed. Copies preserve symlinks and metadata, and cross-device moves fall back to copy then delete.

Plans with deletes or multiple changes ask for confirmation. A single rename or create applies immediately. Permission checks use effective IDs, run in the background while planning and again immediately before execution, and are discarded if the editor quits. A failed check stops the plan before any operation. Checks are advisory: root may bypass restrictions, while ACL changes, read-only mounts, immutable attributes, and races can still cause failures during execution. Execution stops at the first filesystem error and does not roll back operations already applied. The message names the operation, path, and error, lists what was applied, and reloads the listing from disk. Recursive checks for permanent directory deletion stop after 50 ms or 4096 entries so a large tree cannot stall the editor. When no writable trash exists for the target filesystem, preflight refuses and suggests permanent deletion with `directory_trash=false`.

The directory is checked again before applying. If entries changed since the buffer loaded, Kawi refuses the write and asks you to reload.

`:wall` checks every dirty directory buffer and plans their changes together. When the combined plan needs confirmation, Kawi asks once before applying any directory change. If a listing changed on disk or any plan is invalid, no directory changes or file writes are applied. Nested dirty directory buffers must be saved separately, and the refusal names both roots. A uniquely matching delete and create for the same name in different dirty directories is applied as a move. Other deletes follow the trash rule.

## Commands and keys

- `:e dir/` open a directory buffer
- `kawi .` start in the current directory
- `-` open the parent directory from a directory buffer
- `<leader> e -` open the parent directory from any buffer
- `Enter` open the entry under the cursor
- `.` toggle hidden entries when the buffer is clean
- `:w` apply pending changes
- `:w!` apply without the directory confirmation prompt
- `:wall` apply pending changes in all dirty directory buffers as one transaction
- `:wqall` apply all pending changes and quit
- `:bd` close the directory buffer and keep Kawi open
- `:bd!` discard pending listing edits and close the directory buffer

Open buffers whose paths are renamed are retargeted, buffers under deleted paths are closed, and LSP rename notifications are sent after a successful apply.

## Configuration

```toml
[explorer]
show_hidden = false
confirm = true
trash = true
sort = "directories"
metadata = ["permissions", "links", "owner", "group", "size", "modified"]
```

- `show_hidden` includes dotfiles in new directory buffers.
- `confirm` asks before deletes and multi-change plans.
- `trash` sends deletes to the trash. Set it to `false` for permanent deletes.
- `sort` is `directories` for directories first, or `name` for natural name order.
- `metadata` lists the metadata columns to show.

The runtime options are `directory_show_hidden`, `directory_confirm`, `directory_trash`, `directory_sort`, and `directory_metadata`.

Directory buffers restore in workspace sessions by path. Unsaved listing edits are not persisted across sessions.

See [Project explorer](help:explorer), [Keybindings](help:keybindings), and [Sessions](help:sessions) for related behavior.
