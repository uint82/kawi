---
title = "Diagnostics"
aliases = ["diagnostic", "errors"]
related = ["lsp", "configuration", "keybindings"]
---
# Diagnostics

Language server and plugin diagnostics appear as underlines, status counts, virtual text, and entries in the problems list. A plugin with `diagnostics.publish` publishes through `editor.set_diagnostics`, and Kawi labels the source with the plugin name. Gutter signs appear when the sign column is visible.

Plugin diagnostics are scoped to the publishing plugin and the current named buffer. Kawi rejects more than 1000 diagnostics per call, messages over 4096 bytes, codes over 256 bytes, stale buffers or revisions, and invalid positions. Publishing is limited to 20 calls per plugin per second. Kawi clears a plugin's diagnostics when the plugin stops, crashes, is disabled, restarts, or when the buffer closes.

## Keys

- `<leader> d d` show diagnostics on the cursor line
- `<leader> d n` next diagnostic
- `<leader> d p` previous diagnostic
- `<leader> d l` open the diagnostics list
- `]d` and `[d` next and previous diagnostic
- `]q` and `[q` next and previous quickfix entry

## Display

The sign column is disabled by default. Set `signcolumn` to `auto` or `always` to show diagnostic signs.

Diagnostic underlines are curly by default. Set `diagnostic_underline` to `straight` for a plain underline.

Virtual text is enabled for all lines by default. Set `diagnostic_virtual_text` to `off`, `cursor_line`, or `all`.

Set `diagnostic_virtual_severity` to control the minimum shown severity. The default is `warning`.

Diagnostic counts show errors and warnings in the status line. Set `diagnostic_counts` to `all` to include information and hints, and enable `diagnostic_workspace_counts` for an optional workspace total.

Deprecated ranges are struck through and unnecessary ranges are faded.
