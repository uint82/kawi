---
title = "Error codes"
aliases = ["error-codes"]
related = ["diagnostics", "troubleshooting", "configuration"]
---
# Error codes

Built-in errors and warnings start with a stable code. `E` and `W` codes follow Vim's meaning where Kawi implements the same behavior. `K` codes are Kawi-specific. Run `:help CODE` to open the matching entry. The code names the behavior; the wording can change.

## Command errors

### E146: Regular expressions can't be delimited by letters
A substitute pattern uses a letter as its delimiter. Use punctuation such as `/` or `#`.

### E471: Argument required
A known command is missing an argument. Run `:help commands` to check its form.

### E474: Invalid argument
A command rejects the supplied argument. Check the command form and the accepted values.

### E488: Trailing characters
A command received text it does not accept. Remove the extra text or quote the argument.

### E492: Not an editor command
The top-level name is not registered. Check the spelling, run `:commands`, or use `:help <topic>`.

### E493: Backwards range given
The range end is before its start. Reverse the range or use the command form that accepts it.

### E939: Positive count required
A substitute count is zero. Use a count greater than zero.

### E1510: Value too large
A numeric argument does not fit the supported range. Use a smaller value.

## Files and buffers

### E13: File exists
The target file already exists. Add `!` to the command to overwrite it.

### E32: No file name
The buffer has no file name. Save it with `:saveas <path>` first.

### E37: No write since last change
The buffer has unsaved changes. Write it or add `!` to discard the changes.

### E45: 'readonly' option is set
The buffer is read-only. Add `!` to the write command to override the option.

### E89: No write since last change for buffer
A buffer has unsaved changes. Write it or add `!` to discard the changes.

### E94: No matching buffer
No buffer matches the supplied number or name. Run `:ls` to list buffers.

### E149: Sorry, no help for topic
The help index has no page or code for the topic. Run `:help index` to browse the available topics.

### E382: Cannot write, internal buffer
The buffer belongs to the editor. Copy the text to a file buffer before writing.

### E444: Cannot close last window
The editor keeps at least one window open. Open another window before closing this one.

### E484: Can't open file
The file cannot be opened. Check the path and permissions.

### E502: Is a directory
The path is a directory, not a file. Choose a file path.

### E518: Unknown option
The option name is not registered. Run `:help options` to list options.

## Editing state

### E19: Mark has invalid line number
The mark points past the end of the buffer. Set the mark again.

### E20: Mark not set
The mark has not been set. Set it before jumping to it.

### E30: No previous command line
The command-line history is empty. Run a command first.

### E31: No such mapping
The key sequence has no mapping in that mode. Check the mode and the key sequence.

### E33: No previous substitute regular expression
No substitute pattern is available. Run a substitute first, then repeat it.

### E35: No previous regular expression
No search pattern is available. Search first, then repeat the search.

### E86: Buffer does not exist
The buffer number is not open. Run `:ls` to list buffers.

### E93: More than one match
The buffer name matches more than one buffer. Use the buffer number.

### E348: No string under cursor
The cursor is not on a word or string. Move the cursor and try again.

### E354: Invalid register name
The register name is not valid. Use a letter, a number, or a supported special register.

### E486: Pattern not found
The search or substitute pattern did not match. Check the pattern and the range.

### E487: Argument must be positive
The option value must be greater than zero. Use a positive value.

### E748: No previously used register
No register has been used yet. Run a command that fills a register first.

### E830: Undo number not found
The undo sequence does not exist. Run `:undolist` to inspect the available sequences.

## Kawi codes

### K001: Unknown task
The task name is not defined. Run `:task list` to see the configured tasks.

### K002: No previous task
No earlier task is available. Run a task first.

### K003: No task is running
The command needs a running task. Start a task first.

### K004: No task output
The task has not produced output yet. Wait for output or check the task log.

### K005: A task is already running
Stop the current task before starting another one.

### K006: No Git editor session
The Git commit editor is not open. Start a commit first.

### K007: LSP command requires a file or server
The command needs a file or a running language server. Open a file or start a server.

### K008: Language server service is not running
The language server is not available. Run `:lsp status` to check its state.

### K100: No keymap file loaded
No custom keymap file is loaded. Check the keymap configuration.

### K101: Not a directory
The path is not a directory. Choose a directory path.

### K102: Unknown theme
The theme name is not registered. Run `:help themes` to list themes.

### K103: Unknown keymap
The keymap name is not registered. Check the `keymap` option values.

### K104: Invalid theme file
The theme file could not be parsed. Check the TOML syntax and the theme format.
