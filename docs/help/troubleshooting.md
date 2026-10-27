---
title = "Troubleshooting"
aliases = ["problems"]
related = ["configuration", "lsp", "sessions"]
---
# Troubleshooting

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
