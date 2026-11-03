# Changelog

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
