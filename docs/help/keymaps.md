---
title = "Keymaps"
aliases = ["mappings", "custom-keys"]
related = ["keybindings", "configuration", "commands"]
---
# Keymaps

Kawi reads custom key mappings from `$XDG_CONFIG_HOME/kawi/keymaps.toml`, normally `~/.config/kawi/keymaps.toml`.

Mappings bind key sequences to command names. Commands have stable names such as `picker.files`, `edit.delete_line`, and `lsp.definition`. Run `:commands` to list every available command.

## Modes

Mappings live in three tables:

```toml
[normal]
"<leader>ff" = "picker.files"

[insert]
"jk" = "mode.normal"

[visual]
"<leader>fc" = "edit.comment"
```

Replace mode uses the `insert` table. Command mode and search mode are not remappable yet.

## Values

A mapping value is either a command name or a table:

```toml
[normal]
"gx" = "edit.delete_line"
"gX" = { command = "edit.delete_line", desc = "delete the line" }
```

Table fields:

- `command` is required and must be a known command name.
- `desc` overrides the command description in `:map` output.
- `args` passes command arguments. The `keys.send` command uses `args.keys`.
- `when` is accepted for future context predicates. Kawi currently stores it but does not evaluate it.

## SendKeystrokes

Use the `keys.send` command for key-to-key remaps. It feeds the given keys through the normal input pipeline, so it can invoke built-in grammar without recursive mappings.

```toml
[normal]
"Q" = { command = "keys.send", args = { keys = "gq" } }
```

Kawi limits injected sequences to 100 keys and 8 nested `keys.send` calls.

## Leader

`<leader>` expands to every configured leader key. The default leaders are Space and Backslash.

```toml
leader = ","
```

An array configures several leaders:

```toml
leader = [" ", "\\"]
```

A leader must be a single key. Pressing `<leader>` alone opens the built-in leader menu, and configured mappings take precedence over the menu.

## Timeout

When the typed keys are a prefix of a longer mapping, Kawi waits before choosing:

```toml
timeout = 1000
```

The value is milliseconds and defaults to `1000`. A complete mapping wins after the timeout. A prefix with no complete mapping is replayed to the built-in keymap. A longer timeout allows slower multi-key mappings. A shorter timeout makes built-in prefixes such as the leader menu appear sooner.

## Key notation

- Plain characters: `gg`, `gx`, `]d`
- Control: `<C-w>`
- Alt: `<M-x>`
- Shift: `<S-Tab>`
- Named keys: `<Esc>`, `<CR>`, `<Tab>`, `<BS>`, `<Del>`, `<Ins>`, `<Home>`, `<End>`, `<PageUp>`, `<PageDown>`, `<Up>`, `<Down>`, `<Left>`, `<Right>`, `<F1>` to `<F24>`
- Literals: `<Space>`, `<lt>`, `<gt>`, `<bar>`, `<bslash>`
- Leader: `<leader>`

Use `:nmap`, `:imap`, and `:vmap` for runtime mappings. Use `:nunmap`, `:iunmap`, and `:vunmap` to remove a runtime mapping or suppress a built-in mapping. Unmapping an unbound key creates a no-op binding.

## Introspection

- `:commands` lists command names, descriptions, and aliases.
- `:map` lists every configured mapping.
- `:map normal`, `:map insert`, and `:map visual` filter by mode.
- `:map reload` reloads `keymaps.toml` and replaces all runtime mappings with the file contents.

## Precedence

Configured mappings are checked before the built-in keymap. A complete match runs its command. A pending prefix waits for more input or the timeout. An unmatched sequence is replayed to the built-in keymap, so Vim operators, counts, registers, and leader menus keep working. Counts and register prefixes are reserved and cannot start a mapping.

Mappings are command-based and apply to every keymap preset. The preset controls the built-in keys; it does not filter user mappings.

## Limits

- Operator-pending, command, and search mappings are not supported.
- Picker, prompt, and terminal keys are not remappable yet.
- Mappings are command-only. Recursive Vim-style remaps are not supported.
- `when` predicates are reserved for a future release.
