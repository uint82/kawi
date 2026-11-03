---
title = "Troubleshooting"
aliases = ["problems"]
related = ["configuration", "lsp", "sessions"]
---
# Troubleshooting

## Run the doctor

Run `:health` to check configuration, keymaps, plugin keybindings, language servers, tasks, Git, sessions, and state directories without leaving the editor. Use `:health lsp` for language servers and `:lsp status` for the runtime server table.

Run `kawi doctor` to run the same checks from the command line. Use `:plugin list` and `:plugin logs` to diagnose plugins.

Use `kawi doctor --json` for a machine-readable report and `kawi doctor --strict` to fail on warnings. Inspect another project with `kawi doctor --path PATH`.

## A language server does not start

Check `~/.config/kawi/languages.toml`, then confirm the server command exists and the project root marker matches.

## Configuration errors

Configuration warnings are reported at startup. Use `:set <option>?` to inspect a value and `:help option <name>` for its definition.

## Diagnostics look stale

Diagnostics update after the language server publishes new results. Run `:edit` to reload the buffer or restart the affected server.

## Files changed outside Kawi

Open files are watched through their parent directories. Clean buffers reload after external changes, while dirty buffers show `[changed]` and ask before overwriting. An open file renamed or moved within the same filesystem and a watched location retargets its buffer and preserves content, cursor, view, and dirty state. A deleted file or ambiguous move shows `[deleted]` and keeps the buffer open.

## Recovery

If Kawi exits unexpectedly, accept the recovery prompt on the next start. Recovery data lives in the XDG state directory.

## More help

Open [Configuration](help:configuration) or [Language servers](help:lsp).
