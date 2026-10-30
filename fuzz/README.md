# Fuzz workspace

This directory is a separate Cargo workspace. It is excluded from the main workspace and from `cargo vet`, because fuzzing only builds on nightly with `cargo-fuzz` and `libfuzzer-sys`. The vet exclusion for `libfuzzer-sys` is deliberate: it is a build and test harness dependency, not a dependency of the shipped editor or the plugin sandbox.

## Targets

- `network_policy`
- `stream_bytes`
- `process_framing`
- `authorization`
- `host_imports`
- `process_run`

## Corpus and seeds

The corpus is cached by CI. Seeded inputs under `fuzz/corpus/<target>` keep coverage on deep paths: parsing, framing, capability decisions, network classification, byte accounting, and process authorization. New findings are added back to the corpus when they are stable and small.

## Coverage

Coverage is measured at line and region granularity. Branch-only paths are not a gate because the targets call host-side functions directly and some error paths depend on platform behavior. The seeded deep paths are the baseline that the scheduled matrix keeps warm.

## Running

```sh
sh run.sh smoke
sh run.sh nightly <target>
```

`smoke` runs 30 seconds per target. `nightly` defaults to 120 seconds per target and accepts one target name. `FUZZ_SECONDS`, `FUZZ_RSS_MB`, and `FUZZ_MALLOC_MB` override the budgets. Pull requests that touch `crates/kawi-plugin`, `crates/kawi-jsonrpc`, or this directory run the smoke job. The weekly schedule and manual dispatch run all six targets in a parallel matrix and upload crash artifacts.
