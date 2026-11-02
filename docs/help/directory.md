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

Names are stored as `OsString` values. Unchanged lines are never renamed, so existing names remain byte-for-byte intact. Backslashes, control characters, and bytes that are not valid UTF-8 are escaped so each entry occupies exactly one buffer line. Recognized escapes include `\\`, `\a`, `\b`, `\t`, `\n`, `\v`, `\f`, `\r`, and `\xHH`. Editing a line decodes the escapes before applying changes. Use a literal `/` to create or move entries into subdirectories; escaped slashes are rejected.

`Enter` opens the entry under the cursor. Regular files and symlinks to regular files open in a buffer, and symlinks to directories open as directories. FIFOs, sockets, block devices, and character devices are not opened. Broken symlinks report an error instead of creating a buffer. `.` toggles hidden entries when the buffer has no unsaved changes.

The buffer is a normal modified buffer. `:q` closes the window, so with one window it exits Kawi. Use `:bd` to close only the directory buffer. `:q` and `:bd` refuse pending listing edits without `!`. `:e!` discards pending listing edits and reloads from disk. After a successful write the listing reloads from disk and the text undo history is cleared.

## Sorting

Listings are sorted after each load. `sort` selects the key: `name`, `size`, `modified`, `extension`, or `kind`. The legacy value `directories` is accepted as an alias for `name`; grouping is controlled by `sort_directories_first`. Natural name order is always the final tie-breaker, so listings stay deterministic.

```toml
[explorer]
sort = "name"
sort_reverse = false
sort_directories_first = true
```

- `sort_reverse` reverses the selected key. Directories stay grouped first when `sort_directories_first` is enabled.
- `sort_directories_first` groups directories before other entries. It is enabled by default.
- `sort = "size"` orders from smallest to largest. `sort = "modified"` orders from oldest to newest. Use `sort_reverse` for descending order.
- `sort = "extension"` places entries without an extension first, then compares extensions in natural order.
- `sort = "kind"` orders directories, regular files, symlinks, FIFOs, sockets, block devices, and character devices.
- Entries without loaded size or modified metadata sort after entries with that metadata.

Sorting by size or modified reads filesystem metadata even when metadata columns are hidden. Changing a sort option reloads clean open directory buffers immediately and keeps the selected entry. Dirty buffers keep their listing until they are saved or reloaded.

## Live refresh

The root of every open directory buffer is watched while Kawi runs. External creates, deletes, renames, symlink target changes, and metadata updates reload clean directory buffers automatically. Changing `directory_show_hidden`, `directory_sort`, `directory_sort_reverse`, `directory_sort_directories_first`, or `directory_metadata` also reloads clean directory buffers immediately.

- Dirty directory buffers are never overwritten. Their status shows `[changed]`, Kawi warns once, and the marker clears after a reload or successful save. Saves still recheck the directory and refuse if it changed.
- The cursor stays on the selected entry when it still exists. Window and scroll state are preserved.
- Listings produced by Kawi's own writes are compared with the current buffer, so an identical listing does not reload twice.
- A nested directory buffer watches its own root. Changes inside a subdirectory also refresh the parent when that subdirectory's metadata changes.
- If a watched root is removed, recreated, or replaced, Kawi re-arms the watch and reloads the directory.
- A renamed or moved root inside a watched location retargets its directory buffer and nested directory buffers. The listing, cursor, view, options, and open buffer paths are preserved.
- A deleted root, or one moved outside watched locations, is marked `[deleted]` in the status bar and its listing stays readable. Recreating the root and reloading clears the status.
- If filesystem notifications are unavailable, Kawi reports it and `:e` remains the manual refresh path.

## Metadata

Directory metadata is display-only text before each entry name, in permissions, links, owner, group, size, and modified order. Symlink targets are display-only text after the entry name, escaped like names. Neither is part of the buffer, so neither affects editing, search, or writes.

```toml
[explorer]
metadata = ["permissions", "links", "owner", "group", "size", "modified"]
```

- `permissions` shows Unix mode bits with the `ls` type character: `-` for regular files, `d` for directories, `l` for symlinks, `p` for FIFOs, `s` for sockets, `b` for block devices, and `c` for character devices. Other platforms show unknown permissions.
- `links` shows the hard link count. Other platforms show `?`.
- `owner` shows the owner name resolved from the user database, falling back to the numeric ID. Other platforms show `?`.
- `group` shows the group name resolved from the group database, falling back to the numeric ID. Other platforms show `?`.
- `size` shows the raw byte count from the filesystem, including for directories.
- `modified` shows local time in `ls` style, such as `Nov  1 20:26` for recent entries and `Nov  1  2025` for older entries.

All six columns are shown by default. Set `:set directory_metadata=none` or use `metadata = []` to hide metadata. Use a comma-separated list at runtime, such as `:set directory_metadata=permissions,size`. Owner, group, and size column widths grow to fit the widest value in the listing.

Metadata is read with the listing on the background worker. Changing the option reloads clean open directory buffers immediately and leaves dirty buffers untouched until save or reload. Rows without loaded metadata keep their column space blank.

## Safety

Before writing, Kawi validates names, duplicate targets, missing parents, existing targets, directory moves into themselves, moves into a directory that is being renamed, and permissions for affected directories, copy sources, and deletes. Permission checks cover search access on parents, write access on directories moved across parents, sticky-directory ownership, and the trash for the target filesystem. All permission failures are listed together, naming each operation and path. Rename cycles and swaps use temporary names.

Deletes go to the freedesktop trash by default. Unlike symlinks, the link itself is renamed or removed and never followed. Copies preserve symlinks and metadata, and cross-device moves fall back to copy then delete.

Plans with deletes or multiple changes ask for confirmation. A single rename or create applies immediately. Permission checks use effective IDs, run in the background while planning and again immediately before execution, and are discarded if the editor quits. A failed check stops the plan before any operation. Checks are advisory: root may bypass restrictions, while ACL changes, read-only mounts, immutable attributes, and races can still cause failures during execution. Execution stops at the first filesystem error and does not roll back operations already applied. The message names the operation, path, and error, lists what was applied, and reloads the listing from disk. Recursive checks for permanent directory deletion stop after 50 ms or 4096 entries so a large tree cannot stall the editor. When no writable trash exists for the target filesystem, preflight refuses and suggests permanent deletion with `directory_trash=false`.

The directory is checked again before applying. If entries changed since the buffer loaded, Kawi refuses the write and asks you to reload. Writes to a deleted root are refused. Recreate the root and reload, or use `:e!` to discard the pending changes.

`:wall` checks every dirty directory buffer and plans their changes together. When the combined plan needs confirmation, Kawi asks once before applying any directory change. If a listing changed on disk or any plan is invalid, no directory changes or file writes are applied. Nested dirty directory buffers must be saved separately, and the refusal names both roots. A uniquely matching delete and create for the same name in different dirty directories is applied as a move. Other deletes follow the trash rule.

## Commands and keys

- `:e dir/` open a directory buffer
- `:e!` discard pending listing edits and reload
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
sort = "name"
sort_reverse = false
sort_directories_first = true
metadata = ["permissions", "links", "owner", "group", "size", "modified"]
```

- `show_hidden` includes dotfiles in new directory buffers.
- `confirm` asks before deletes and multi-change plans.
- `trash` sends deletes to the trash. Set it to `false` for permanent deletes.
- `sort` selects `name`, `size`, `modified`, `extension`, or `kind`. The legacy value `directories` is accepted as an alias for `name`.
- `sort_reverse` reverses the selected sort key.
- `sort_directories_first` groups directories before other entries.
- `metadata` lists the metadata columns to show.

The runtime options are `directory_show_hidden`, `directory_confirm`, `directory_trash`, `directory_sort`, `directory_sort_reverse`, `directory_sort_directories_first`, and `directory_metadata`.

Directory buffers restore in workspace sessions by path. Unsaved listing edits are not persisted across sessions.

See [Project explorer](help:explorer), [Keybindings](help:keybindings), and [Sessions](help:sessions) for related behavior.
