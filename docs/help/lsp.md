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

Use `K` for hover and `<leader> c a` for code actions. See [Diagnostics](help:diagnostics) for diagnostic navigation and display.
