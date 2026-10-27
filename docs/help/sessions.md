---
title = "Sessions and recovery"
aliases = ["session", "recovery", "restore"]
related = ["configuration", "troubleshooting"]
---
# Sessions and recovery

Kawi stores workspace sessions and restores them when it starts without a file argument.

Recovery snapshots protect unsaved changes. If a session crashes or exits unexpectedly, Kawi offers recovery on the next start.

Autosave is configurable:

```toml
[editor]
autosave = "off"
autosave_delay = 500
```

Valid autosave modes are `off`, `idle`, `focus`, and `all`. Undo history can also persist across sessions with `undo_persist`.

Use `:help option autosave` or `:help option undo_persist` for details.
