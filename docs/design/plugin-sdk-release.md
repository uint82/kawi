# Plugin SDK release

Status: automated by the `release-sdk` workflow.

## Setup

Set the `CARGO_REGISTRY_TOKEN` repository secret to a crates.io API token with publish access to `kawi-plugin-sdk`.

## Versioning

`kawi-plugin-sdk` is versioned independently from the editor. The SDK version and the WIT package version must match. The current WIT package is `kawi:plugin@0.5.0`. Interface changes bump the WIT package, and components built against an earlier package must be rebuilt.

## Checklist

1. Update `crates/kawi-plugin-sdk/Cargo.toml`, the root `Cargo.toml`, `examples/wasm-trim/Cargo.toml`, `crates/kawi/src/plugin_cli/scaffold.rs`, the package line in `crates/kawi-plugin-sdk/wit/plugin.wit`, the host import policy, the WAT fixtures, and documentation that pins the current version.
2. Rebuild the WASM fixtures and trim example, refresh `Cargo.lock`, and confirm the SDK and WIT package versions match.
3. Run `cargo publish --dry-run -p kawi-plugin-sdk --locked`.
4. Run `git diff --exit-code -- crates/kawi-plugin/tests/fixtures/wasm examples/wasm-trim` after a rebuild.
5. Merge the release change.
6. Tag `kawi-plugin-sdk-v<version>` on `main` and push the tag. The workflow publishes the crate and builds the scaffold from crates.io.
7. Confirm the workflow published the version and the published scaffold test passed.

## Rollback

Yank the release with `cargo yank --version <version> kawi-plugin-sdk` and publish a patch version. Do not reuse the yanked version.
