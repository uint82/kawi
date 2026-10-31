---
title = "Troubleshooting"
aliases = ["problems"]
related = ["configuration", "lsp", "sessions"]
---
# Troubleshooting

## Run the doctor

Run `kawi doctor` to check configuration, keymaps, plugin keybindings, language servers, tasks, Git, sessions, and state directories. Use `:plugin list` and `:plugin logs` to diagnose plugins.

Use `kawi doctor --json` for a machine-readable report and `kawi doctor --strict` to fail on warnings. Inspect another project with `kawi doctor --path PATH`.

## A language server does not start

Check `~/.config/kawi/languages.toml`, then confirm the server command exists and the project root marker matches.

## Configuration errors

Configuration warnings are reported at startup. Use `:set <option>?` to inspect a value and `:help option <name>` for its definition.

## Diagnostics look stale

Diagnostics update after the language server publishes new results. Run `:edit` to reload the buffer or restart the affected server.

## Recovery

If Kawi exits unexpectedly, accept the recovery prompt on the next start. Recovery data lives in the XDG state directory.

## More help

Open [Configuration](help:configuration) or [Language servers](help:lsp).
