# Changelog

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
