---
title = "Debugging"
aliases = ["dap", "debugger"]
related = ["lsp", "health", "features", "troubleshooting"]
---
# Debugging

Kawi includes a Debug Adapter Protocol client. Debug adapters and launch configurations are declared in `~/.config/kawi/debug.toml`. A project can override them with `.kawi/debug.toml` in the workspace root.

```toml
[adapter.dlv]
command = "dlv"
args = ["dap"]

[configuration.rust]
adapter = "dlv"
request = "launch"
program = "${workspace}/target/debug/myapp"
args = []
cwd = "${workspace}"
```

`request` must be `launch` or `attach`. Fields other than `adapter`, `languages`, and `request` are passed to the adapter as launch or attach arguments. The optional `languages` list selects a configuration from the current buffer language.

String values support `${workspace}`, `${file}`, `${fileDirname}`, and `${env:NAME}`. Adapter entries support `command`, `args`, `env`, and `cwd`.

## Session commands

- `:debug start [configuration]` starts a session. Without a name, Kawi uses the first configuration matching the buffer language.
- `:debug restart` uses the adapter restart request when supported, otherwise it stops the session and starts the last configuration again.
- `:debug stop` disconnects and terminates the debuggee.
- `:debug terminate` asks the adapter to terminate the debuggee without disconnecting first.
- `:debug panel` opens, focuses, or closes the debug panel.
- `:debug continue` resumes execution.
- `:debug next` steps over, `:debug step` steps in, and `:debug out` steps out.
- `:debug status` shows the session state, thread, frame, and breakpoint count.
- `:debug configurations` lists configured launch and attach targets.
- `:debug output` shows adapter and debuggee output.

## Breakpoints

- `:debug breakpoint` toggles a line breakpoint in the current buffer.
- `:debug breakpoint condition <expression>` sets a condition on the current line.
- `:debug breakpoint hit <count>` sets a hit count on the current line.
- `:debug breakpoint log <message>` sets a log message on the current line.
- `:debug breakpoint clear` removes conditions, hit counts, and log messages from the current line.
- `:debug breakpoints` lists all line breakpoints and their options.
- `:debug function <name>` toggles a function breakpoint.
- `:debug functions` lists function breakpoints.
- `:debug exceptions` lists exception filters reported by the adapter.
- `:debug exception <filter>` enables or disables an exception filter.

A filled circle marks a verified breakpoint, a hollow circle marks an unverified breakpoint, and an arrow marks the current stopped line. Breakpoints remain available when a session stops and are sent to the next adapter on launch.

Conditional, hit-count, and log breakpoints depend on adapter capabilities. Function breakpoints require adapter support. Exception filters use the filters advertised by the adapter during initialization.

## Call stacks and variables

- `:debug stack` shows the current call stack.
- `:debug variables` shows every scope for the current frame.
- `:debug expand <reference>` shows nested variables for a reference listed in a variables report.
- `:debug more <reference>` loads the next page of variables for an expanded reference.
- `:debug set <reference> <name> <value>` updates a variable through the adapter.
- `:debug frame <id>` selects a stack frame, jumps to its source location, and refreshes its scopes.
- `:debug threads` lists threads and marks the current thread.
- `:debug thread <id>` selects a thread and refreshes the stack when stopped.

Variable and scope entries include a `[ref N]` marker when they can be expanded. Adapters that report variable counts are queried in pages of 100 entries. Reports show how many entries are loaded and how to load more.

## Debug panel

The debug panel opens automatically when a session starts and can be toggled with `:debug panel`. It shows the session status, threads, call stack, scopes and variables, watches, breakpoints, function breakpoints, exception filters, and console output.

- `j` and `k` move the selection.
- `Ctrl+d`, `Ctrl+u`, `PageDown`, and `PageUp` scroll by a page.
- `g` and `G` jump to the first or last entry.
- `Enter` selects a thread or frame, expands a variable, loads more variables, toggles an exception filter, or jumps to a breakpoint.
- `r` refreshes threads.
- `q` or `Escape` returns focus to the editor.
- `x` closes the panel.

## Console and watches

- `:debug eval <expression>` evaluates an expression and adds it to the debug console.
- `:debug console` shows evaluation history.
- `:debug watch add <expression>` adds a watch expression.
- `:debug watch edit <index> <expression>` replaces a watch expression by its report index.
- `:debug watch remove <expression>` removes a watch expression.
- `:debug watches` lists watch expressions and their latest values.

Watches are evaluated automatically with the current frame whenever execution stops.

## Terminal requests

Kawi supports the DAP `runInTerminal` reverse request for integrated terminals. When an adapter asks to launch the debuggee in a terminal, Kawi opens the integrated terminal, applies the requested working directory and environment, and returns the child process ID to the adapter. External terminal requests and the `startDebugging` reverse request are not supported.

## Built-in commands

The commands `debug.start`, `debug.restart`, `debug.stop`, `debug.terminate`, `debug.panel`, `debug.continue`, `debug.next`, `debug.step_in`, `debug.step_out`, `debug.breakpoint`, `debug.stack`, `debug.variables`, `debug.threads`, `debug.functions`, `debug.exceptions`, `debug.console`, `debug.watches`, and `debug.status` are available for keymaps and plugins.

## Status and health

The status line shows `dap: starting`, `dap: running`, `dap: stopped`, `dap: terminated`, or `dap: failed` while a session exists. Use `:debug status` for details.

Run `:health debug` to validate configured adapters and configurations. The check reports missing executables and configuration warnings.

## Current limits

Kawi does not yet implement data breakpoints, instruction breakpoints, memory reads or writes, exception filter options, or multi-session `startDebugging`. External terminal requests remain unsupported.
