## 0.11.0 - 2026-11-15

### Breaking

- Bump the SDK and WIT package to 0.11.0 (`kawi:plugin@0.11.0`).
- Components built against `kawi:plugin@0.10.0` must be rebuilt with this release.
- The `callback` variant gains a `timer(timer-fired)` case. Update exhaustive matches in `on-callback`.
- Process plugins remain on protocol minor 12. The new interfaces, host methods, and timer callbacks are available only to WebAssembly components.

### Added

- Add the `editor-schedule` interface with one-shot and repeating timers, cancellation, and listing.
- Add the `editor-variables` interface for global, plugin, buffer, and window variables.
- Add the `options-write` interface for setting and resetting editor options.
- Add timer callbacks through `callback-handler.on-callback`.
- Add `editor_schedule`, `editor_variables`, and `options_write` re-exports.

## 0.10.0 - 2026-11-15

### Breaking

- Bump the SDK and WIT package to 0.10.0 (`kawi:plugin@0.10.0`).
- Components built against `kawi:plugin@0.9.0` must be rebuilt with this release.
- Add the exported `callback-handler` interface and its required `on-callback` function. Existing components must implement it.
- Process plugins remain on protocol minor 12. The new interfaces, host methods, and events are available only to WebAssembly components.

### Added

- Add the `editor-autocmds` interface for registering, removing, listing, and clearing asynchronous autocmds, plus `emit-user-event`.
- Add typed autocmd callback events so the host can invoke a registered plugin with the event payload after an event matches.
- Add the `window-created`, `window-closed`, and `language-changed` autocmd events. These events are not delivered through `on-event` in 0.10.
- Add the `user` autocmd event for host-emitted user events.
- Add the `editor_autocmds` re-export.

## 0.9.0 - 2026-11-15

### Breaking

- Bump the SDK and WIT package to 0.9.0 (`kawi:plugin@0.9.0`).
- Components built against `kawi:plugin@0.8.0` must be rebuilt with this release.
- Process plugins remain on protocol minor 12. The new interfaces and host methods are available only to WebAssembly components.

### Added

- Add the `editor-commands` interface for registering, unregistering, and listing runtime commands with aliases.
- Add the `editor-keymaps` interface for setting, removing, and listing runtime key bindings in normal, insert, and visual modes.
- Add `editor_commands` and `editor_keymaps` re-exports.
- Remove runtime commands and key bindings when a plugin stops, restarts, or is disabled.

## 0.8.0 - 2026-11-14

### Breaking

- Bump the SDK and WIT package to 0.8.0 (`kawi:plugin@0.8.0`).
- Components built against `kawi:plugin@0.7.0` must be rebuilt with this release.
- Process plugins remain on protocol minor 12. The new host methods and events are available only to WebAssembly components.

### Added

- Add `editor-buffers` and `editor-buffers-write` interfaces for listing, reading, opening, and editing buffers. `editor-buffers-write` is scoped by the manifest `editor.buffers.write` path list.
- Add the `editor-windows` interface for listing, inspecting, focusing, opening, and closing windows.
- Add the `editor-control` interface for setting the active buffer cursor, selection, and mode.
- Add the `options-read` interface for listing and reading editor options.
- Add `buffer-enter`, `buffer-leave`, `window-enter`, `window-leave`, `cursor-moved`, `selection-changed`, `focus-gained`, and `focus-lost` events.
- Add `BUFFER_NOT_FOUND`, `BUFFER_NOT_ACTIVE`, `WINDOW_NOT_FOUND`, `LAST_WINDOW`, `OPTION_NOT_FOUND`, and `MODE_NOT_SUPPORTED` error codes with matching `ErrorKind` variants.

## 0.7.0 - 2026-11-03

### Breaking

- Bump the SDK and WIT package to 0.7.0 (`kawi:plugin@0.7.0`).
- Components built against `kawi:plugin@0.6.0` must be rebuilt with this release.

### Added

- Add the `ui-panel` interface for opening, updating, focusing, and closing a read-only docked text panel.
- Add `ui_panel` re-exports with typed panel records and optional line styles.
- Add `PANEL_NOT_FOUND` and `ErrorKind::PanelNotFound` for panel ownership and lifetime errors.

## 0.6.0 - 2026-11-03

### Breaking

- Bump the SDK and WIT package to 0.6.0 (`kawi:plugin@0.6.0`).
- Components built against `kawi:plugin@0.5.0` must be rebuilt with this release.

### Added

- Add the `process-jobs` interface for opening, writing to, signaling, killing, polling, and inspecting long-running jobs.
- Add `process_jobs` re-exports with typed job events and status.
- Queue WASM job output and exit events for `process_jobs::poll` because WASM plugins are passive.

## 0.5.0 - 2026-11-01

### Breaking

- Bump the SDK and WIT package to 0.5.0 (`kawi:plugin@0.5.0`).
- Components built against `kawi:plugin@0.4.0` must be rebuilt with this release.
- Replace decoration style enums with manifest-declared decoration groups.

### Added

- Add named decoration groups for highlights, virtual text, and signs.
- Add theme overrides for plugin decoration groups with fill modes and text attributes.

## 0.4.0 - 2026-11-01

### Breaking

- Bump the SDK and WIT package to 0.4.0 (`kawi:plugin@0.4.0`).
- Components built against `kawi:plugin@0.3.0` must be rebuilt with this release.

### Added

- Add the `editor-decorations` interface for buffer highlights and virtual text.
- Add the `editor-signs` interface for gutter signs.
- Add the `ui-statusline` interface for status line segments.
- Re-export `editor_decorations`, `editor_signs`, and `ui_statusline`.

## 0.3.0 - 2026-11-01

### Breaking

- Bump the SDK and WIT package to 0.3.0 (`kawi:plugin@0.3.0`).
- Components built against `kawi:plugin@0.2.0` must be rebuilt with this release.

### Added

- Add the `ui-prompt` interface for text input, selection, and confirmation.
- Add the `ui-progress` interface for cancellable progress reporting.
- Add `ui_prompt` and `ui_progress` re-exports with typed prompt outcomes.
- Add the `UI_BUSY` error code and `ErrorKind::Busy`.

## 0.2.0 - 2026-11-01

### Breaking

- Bump the SDK and WIT package to 0.2.0 (`kawi:plugin@0.2.0`).
- Components built against `kawi:plugin@0.1.0` must be rebuilt with this release.

### Added

- Add the `editor-execute` interface for allowlisted built-in editor commands.
- Add `editor_execute` re-exports for command execution and discovery.
- Add the `config-read` interface for plugin settings.
- Add `config_read` re-exports for settings lookup.

## 0.1.0

- Initial public release.
- Versioned `kawi:plugin@0.1.0` bindings.
- UTF-8 position helpers.
- Stale-revision retry helper.
- Protocol error codes and error classification.
