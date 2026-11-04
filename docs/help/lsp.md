---
title = "Language servers"
aliases = ["servers"]
related = ["diagnostics", "configuration", "features", "formatting"]
---
# Language servers

Kawi starts language servers configured in `~/.config/kawi/languages.toml`.

Kawi includes built-in definitions for many languages. Systems languages include Rust, C, C++, Objective-C, Go, Zig, Nim, D, Swift, Crystal, GLSL, WGSL, Assembly, Verilog, SystemVerilog, VHDL, and Solidity. Web languages include JavaScript, TypeScript, JSX, TSX, HTML, CSS, SCSS, Less, JSON, JSONC, YAML, TOML, XML, GraphQL, Prisma, Vue, Svelte, and Astro. Scripting languages include Python, Ruby, PHP, Perl, Lua, Bash, Fish, Elixir, Erlang, Julia, R, and Awk. JVM and .NET languages include Java, Kotlin, Scala, Clojure, C#, and F#. Functional languages include Haskell, OCaml, Elm, PureScript, Racket, Common Lisp, and Gleam. Data and infrastructure languages include SQL, Protobuf, Dockerfile, Terraform, HCL, Nix, CMake, Just, systemd, Pascal, and Fortran. Documentation languages include Markdown, LaTeX, BibTeX, and Typst.

Each language entry defines the language id, file extensions, root markers, indentation, and the servers to start. A server entry defines its command, arguments, environment, and optional initialization options. Servers are resolved from `PATH`; install the server for a language to enable it. Run `:lsp status` or `:health lsp` to see missing servers.

Language servers provide:

- completion and signature help
- hover documentation
- diagnostics and related information
- document and workspace symbols
- rename and code actions
- formatting when no external formatter is configured
- workspace edits and file operations

## Completion

Use Tab to move down and Shift-Tab to move up. Up and Down, Ctrl-N and Ctrl-P, PageUp and PageDown also navigate the completion list. Selection wraps at both ends.

Press Enter to accept the selected item and Esc to dismiss the list. The menu shows at most ten items and scrolls the selection into view.

Use `K` for hover and `<leader> c a` for code actions. See [Diagnostics](help:diagnostics) for diagnostic navigation and display.

## Lifecycle

Use `:lsp start`, `:lsp stop`, and `:lsp restart` to control language servers without leaving the editor.

- `:lsp stop` stops every server used by the current buffer.
- `:lsp start` starts every server used by the current buffer.
- `:lsp restart` stops and starts every server used by the current buffer.
- Add a server name to target that server across all project roots, for example `:lsp restart rust-analyzer`.

A stopped server stays stopped for its project root until you start or restart it. Opening or changing a buffer does not restart it. The commands require either a file buffer or a server name.

## Status

Run `:lsp status` to open a read-only internal buffer listing every configured server with its state, languages, current-buffer attachment, and command. The `:lsp info` alias shows the same report. The `CURRENT` column marks servers used by the active buffer. States include `ready`, `starting`, `initializing`, `idle`, `missing`, `failed`, `stopped`, `disabled`, and `unused`. Configuration warnings appear above the table. Active progress appears in a `PROGRESS` section, and resolved binary paths and failure messages appear below it. Server state persists when you switch buffers.

Run `:health lsp` for the full language server health check. The health check validates configuration and binaries; `:lsp status` shows runtime lifecycle errors and server messages.
