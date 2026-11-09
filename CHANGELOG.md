# Changelog

All notable changes to Kawi are documented in this file.

## 0.1.0-alpha.1 - 2026-11-10

First public alpha.

### Added

- Native macOS support for PTYs, clipboard, Trash, file watching, state directories, and directory sync.
- Linux and macOS release archives for x86_64 and arm64.
- Modal editing with Vim keymaps, multiple windows, and splits.
- Language servers, external formatters, diagnostics, symbols, rename, formatting, and code actions.
- Fuzzy pickers for files, buffers, live grep, replace, and symbols.
- Project explorer, directory buffers, sessions, recovery, autosave, macros, and persistent undo.
- Git status, staging, commits, branches, hunks, and merge conflict resolution.
- Project tasks and integrated terminal panels.
- Managed WebAssembly plugins with checksum verification, a pinned lockfile, and a published Rust SDK.
- Custom keymaps, themes, configuration, and built-in help.

### Known limitations

- macOS release binaries are unsigned and may need the quarantine attribute cleared.
- Linux release binaries require glibc 2.39 or newer.
- Process plugins run without an OS sandbox on macOS.
- Files deleted to `~/.Trash` cannot use Finder's Put Back.
- Windows is not supported.
