---
title = "Themes"
aliases = ["theme", "colorscheme", "colors"]
related = ["configuration"]
---
# Themes

Kawi includes these themes:

- `default`
- `classic`
- `cobalt`
- `one-dark`
- `gruvbox-dark`
- `gruvbox-light`

Set a theme with `:set theme <name>` or `:set theme=<name>`. Press Tab after `theme` to complete built-in and custom theme names. Use `:help option theme` to list the current values.

## Custom themes

Place theme files in `$XDG_CONFIG_HOME/kawi/themes`, normally `~/.config/kawi/themes`. The file name is the theme name. For example, `personal.toml` is selected with `:set theme personal`.

Custom themes use the same TOML format as the built-in themes. See [Configuration](help:configuration) for the config file location and use `:help options` for the generated option reference.

Press Tab after `:set theme` to discover every built-in and custom theme that is currently valid.

## Status line colors

Status line segments derive their colors from `status`, `diagnostics`, `git`, `selection`, and the base palette. Colored segments use a filled background badge, with the base background as the default foreground. Override individual segments with a `[statusline]` table:

```toml
[statusline.mode.normal]
foreground = "#1a1a1a"
background = "#7aa2f7"

[statusline.diagnostics.error]
foreground = "#ff7e87"
background = "#232f36"
```

The optional keys are `mode.normal`, `mode.insert`, `mode.replace`, `mode.visual`, `mode.command`, `mode.search`, `diagnostics.error`, `diagnostics.warning`, `diagnostics.information`, `diagnostics.hint`, `modified`, `readonly`, `git`, `lsp`, `selection`, `macro_recording`, `filetype`, `indent`, `encoding`, `eol`, `percent`, and `position`. Omitted keys use colors derived from the rest of the theme.
