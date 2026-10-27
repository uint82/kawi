---
title = "Themes"
aliases = ["theme", "colorscheme", "colors"]
related = ["configuration"]
---
# Themes

Kawi includes these themes:

- `default`
- `cobalt`
- `one-dark`
- `gruvbox-dark`
- `gruvbox-light`

Set a theme with `:set theme <name>` or `:set theme=<name>`. Press Tab after `theme` to complete built-in and custom theme names. Use `:help option theme` to list the current values.

## Custom themes

Place theme files in `$XDG_CONFIG_HOME/kawi/themes`, normally `~/.config/kawi/themes`. The file name is the theme name. For example, `personal.toml` is selected with `:set theme personal`.

Custom themes use the same TOML format as the built-in themes. See [Configuration](help:configuration) for the config file location and use `:help options` for the generated option reference.

Press Tab after `:set theme` to discover every built-in and custom theme that is currently valid.
