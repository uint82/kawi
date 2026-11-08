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

## Theme packages

A theme package is data. It is not a plugin, it does not run code, and it cannot register itself through the plugin API. Kawi loads theme packages from the themes directory only.

A package can be a single TOML file such as `personal.toml`, or a directory named after the theme that contains `theme.toml`:

```text
~/.config/kawi/themes/personal/theme.toml
```

The directory form is the package format for themes that ship additional data alongside the theme. The theme name is the directory name. A single file with the same name takes precedence when both exist.

Packages may declare optional distribution metadata:

```toml
[package]
name = "personal"
version = "1.2.0"
description = "A warm palette"
author = "Ada"
homepage = "https://example.com/personal"

[base]
foreground = "#e7e2d6"
background = "#10171a"

[search_match]
foreground = "#10171a"
background = "#f2c27b"
```

`name` and `version` are required when the table is present. The other fields are optional. Unknown package fields and unknown theme fields are rejected.

## Special characters

Non-printable Unicode characters are displayed as placeholders such as `<feff>`, `<200b>`, and `<202e>`. Set the top-level `special_key` color to style those placeholders:

```toml
special_key = "#928374"
```

When omitted, `special_key` uses `gutter.non_text`. The built-in themes define a dedicated color, including the Gruvbox values used by Neovim's `SpecialKey` group.

## Decorations

Plugins and built-in features use six decoration styles: `accent`, `info`, `success`, `warning`, `error`, and `muted`. Override them in a theme package:

```toml
[decorations.info]
foreground = "#10171a"
background = "#6ed0f0"

[decorations.warning]
foreground = "#10171a"
background = "#f2c27b"
```

Omitted styles derive their colors from the rest of the theme.

### Plugin decoration groups

Plugins declare named decoration groups in their manifest. Override a group with `[plugin_decorations.<plugin>.<group>]`:

```toml
[plugin_decorations.markers.todo]
foreground = "#10171a"
background = "#6ed0f0"
fill = "both"
italic = true

[plugin_decorations.markers.todo_text]
foreground = "#6ed0f0"
fill = "foreground"
italic = true
```

The optional keys are `foreground`, `background`, `fill`, `bold`, `italic`, `underline`, and `hidden`. `fill` accepts `both`, `foreground`, or `background`. `hidden` conceals the text while preserving its background. Hidden text is revealed when the cursor is on it. Omitted fields inherit the group defaults and the semantic `[decorations]` colors. Overrides for plugins or groups that are not installed are ignored.

## Completion colors

Completion menus color each item by its language server kind. Omitted kinds derive their colors from the theme syntax colors, with `text` using the base foreground and `file`, `folder`, `reference`, and `event` using `gutter.non_text`. Override individual kinds with a `[completion]` table:

```toml
[completion]
function = "#6ed0f0"
keyword = "#ff7e87"
type_parameter = "#f2c27b"
```

The keys are the language server completion kinds in snake case: `text`, `method`, `function`, `constructor`, `field`, `variable`, `class`, `interface`, `module`, `property`, `unit`, `value`, `enum`, `keyword`, `snippet`, `color`, `file`, `reference`, `folder`, `enum_member`, `constant`, `struct`, `event`, `operator`, and `type_parameter`.

## Status line colors

Status line segments derive their colors from `status`, `diagnostics`, `git`, `selection`, and the base palette. Colored segments use a filled background badge with horizontal padding, and the base background as the default foreground. Padding is dropped before the mode label is shortened. Override individual segments with a `[statusline]` table:

```toml
[statusline.mode.normal]
foreground = "#1a1a1a"
background = "#7aa2f7"

[statusline.diagnostics.error]
foreground = "#ff7e87"
background = "#232f36"
```

The optional keys are `mode.normal`, `mode.insert`, `mode.replace`, `mode.visual`, `mode.command`, `mode.search`, `diagnostics.error`, `diagnostics.warning`, `diagnostics.information`, `diagnostics.hint`, `modified`, `readonly`, `git`, `lsp`, `selection`, `macro_recording`, `filetype`, `indent`, `encoding`, `eol`, `percent`, and `position`. Omitted keys use colors derived from the rest of the theme.
