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

## Commands

- `:debug start [configuration]` starts a session. Without a name, Kawi uses the first configuration matching the buffer language.
- `:debug stop` disconnects and terminates the debuggee.
- `:debug continue` resumes execution.
- `:debug next` steps over, `:debug step` steps in, and `:debug out` steps out.
- `:debug breakpoint` toggles a line breakpoint in the current buffer.
- `:debug breakpoints` lists all breakpoints.
- `:debug stack` shows the current call stack.
- `:debug variables` shows variables from the first scope of the current frame.
- `:debug output` shows adapter and debuggee output.
- `:debug status` shows the session state, thread, frame, and breakpoint count.
- `:debug configurations` lists configured launch and attach targets.

The built-in commands `debug.start`, `debug.stop`, `debug.continue`, `debug.next`, `debug.step_in`, `debug.step_out`, `debug.breakpoint`, `debug.stack`, `debug.variables`, and `debug.status` are available for keymaps and plugins.

## Breakpoints

Breakpoints are stored per file and line. A filled circle marks a verified breakpoint, a hollow circle marks an unverified breakpoint, and an arrow marks the current stopped line. Breakpoints remain available when a session stops and are sent to the next adapter on launch.

## Status

The status line shows `dap: starting`, `dap: running`, `dap: stopped`, `dap: terminated`, or `dap: failed` while a session exists. Use `:debug status` for details.

Run `:health debug` to validate configured adapters and configurations. The check reports missing executables and configuration warnings.

## Current limits

Kawi currently implements launch and attach, line breakpoints, stepping, call stacks, variables, output, and termination. Evaluate, watches, conditional breakpoint editing, function breakpoints, exception filters, and adapters that require the `runInTerminal` reverse request are not supported yet.
