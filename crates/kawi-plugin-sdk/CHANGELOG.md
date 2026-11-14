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
