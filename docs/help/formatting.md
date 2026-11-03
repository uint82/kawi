---
title = "Formatting"
aliases = ["formatters", "formatter"]
related = ["lsp", "configuration", "health"]
---
# Formatting

Run `:format`, `<leader> c f`, or enable `format_on_save` to format the current buffer.

Kawi uses the external formatter configured for the language when one is available. Otherwise it asks the language server. External formatters format the whole buffer, even when a visual selection is active. Formatting is recorded as one undo step.

## Configure a formatter

Add a `formatter` table to a language in `~/.config/kawi/languages.toml`.

```toml
[[language]]
name = "rust"
formatter = { command = "rustfmt" }

[[language]]
name = "typescript"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }
```

The formatter table supports these keys:

- `command` is required and names an executable or an absolute path.
- `args` is an optional argument list. `{path}` is replaced with the buffer path.
- `env` is an optional table of environment variables.
- `stdin` defaults to `true`. Kawi pipes the buffer through stdin and reads stdout. Set `stdin = false` to write a temporary file, replace `{path}` with that file, and expect the formatter to rewrite it in place.

Set `formatter = false` to disable formatting for a language.

## Examples

```toml
[[language]]
name = "rust"
formatter = { command = "rustfmt" }

[[language]]
name = "go"
formatter = { command = "gofmt" }

[[language]]
name = "c"
formatter = { command = "clang-format", args = ["-assume-filename={path}"] }

[[language]]
name = "cpp"
formatter = { command = "clang-format", args = ["-assume-filename={path}"] }

[[language]]
name = "python"
formatter = { command = "ruff", args = ["format", "-"] }

[[language]]
name = "javascript"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }

[[language]]
name = "typescript"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }

[[language]]
name = "json"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }

[[language]]
name = "css"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }

[[language]]
name = "html"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }

[[language]]
name = "yaml"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }

[[language]]
name = "markdown"
formatter = { command = "prettier", args = ["--stdin-filepath", "{path}"] }

[[language]]
name = "toml"
formatter = { command = "taplo", args = ["fmt", "-"] }

[[language]]
name = "lua"
formatter = { command = "stylua", args = ["-"] }

[[language]]
name = "bash"
formatter = { command = "shfmt" }
```

## Format on save

Enable format-on-save in `config.toml`:

```toml
[editor]
format_on_save = true
```

You can also toggle it with `:set format_on_save` or query it with `:set format_on_save?`.

If the formatter fails, Kawi saves the buffer without formatting and reports the reason. If the formatter command is not installed, Kawi falls back to the language server.

## Health

Run `:health languages` to list configured formatters, resolved binary paths, and missing commands. The main `:health` report includes the same check.
