---
title = "Language servers"
aliases = ["servers"]
related = ["diagnostics", "configuration", "features"]
---
# Language servers

Kawi starts language servers configured in `~/.config/kawi/languages.toml`.

Each language entry defines the language id, file extensions, root markers, and the servers to start. A server entry defines its command, arguments, environment, and optional initialization options.

Language servers provide:

- completion and signature help
- hover documentation
- diagnostics and related information
- document and workspace symbols
- rename and code actions
- formatting
- workspace edits and file operations

## Completion

Use Tab to move down and Shift-Tab to move up. Up and Down, Ctrl-N and Ctrl-P, PageUp and PageDown also navigate the completion list. Selection wraps at both ends.

Press Enter to accept the selected item and Esc to dismiss the list. The menu shows at most ten items and scrolls the selection into view.

Use `K` for hover and `<leader> c a` for code actions. See [Diagnostics](help:diagnostics) for diagnostic navigation and display.

## Status

Run `:lsp status` to list every configured server with its state, languages, and command. The `:lsp info` alias shows the same table. States include `ready`, `starting`, `initializing`, `idle`, `missing`, `failed`, `stopped`, `disabled`, and `unused`. Configuration warnings appear above the table. Resolved binary paths and failure messages appear below it. Server state persists when you switch buffers.

Run `:health lsp` for the full language server health check.
