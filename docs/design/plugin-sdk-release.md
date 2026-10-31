# Plugin SDK release

Status: automated by the `release-sdk` workflow.

## Setup

Set the `CARGO_REGISTRY_TOKEN` repository secret to a crates.io API token with publish access to `kawi-plugin-sdk`.

## Versioning

`kawi-plugin-sdk` is versioned independently from the editor. The SDK version and the WIT package version must match. The current WIT package is `kawi:plugin@0.1.0`.

## Checklist

1. Update `crates/kawi-plugin-sdk/Cargo.toml`, `Cargo.toml`, `examples/wasm-trim/Cargo.toml`, `crates/kawi/src/plugin_cli/scaffold.rs`, the package line in `crates/kawi-plugin-sdk/wit/plugin.wit`, and any documentation that pins the current version.
2. Refresh `Cargo.lock` and confirm the SDK version and WIT package version match.
3. Run `cargo publish --dry-run -p kawi-plugin-sdk --locked`.
4. Merge the release change.
5. Tag `kawi-plugin-sdk-v<version>` on `main` and push the tag. The workflow publishes the crate and builds the scaffold from crates.io.
6. Confirm the workflow published the version and the published scaffold test passed.

## Rollback

Yank the release with `cargo yank --version <version> kawi-plugin-sdk` and publish a patch version. Do not reuse the yanked version.
