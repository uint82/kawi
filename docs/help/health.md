---
title = "Health"
aliases = ["checkhealth", "healthcheck", "doctor"]
related = ["lsp", "debug", "troubleshooting", "configuration"]
---
# Health

Run `:health` to inspect configuration, paths, language servers, debug adapters, tasks, Git, sessions, plugins, and state directories. The report opens in a read-only internal buffer.

Each check reports a level of `ok`, `info`, `warn`, or `error` with details and an optional hint. The summary counts every level.

Filter the report with a check name:

- `:health lsp` or `:health languages` language servers
- `:health debug` debug adapters and configurations
- `:health config` configuration
- `:health paths` configuration and state paths
- `:health keymaps` keymaps and mappings
- `:health plugins` plugins
- `:health tasks` tasks
- `:health git` Git
- `:health session` sessions
- `:health state` state directories

`kawi doctor` runs the same checks from the command line. Use `kawi doctor --json` for a machine-readable report and `kawi doctor --strict` to fail on warnings.
