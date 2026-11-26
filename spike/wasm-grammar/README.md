# WASM grammar spike

This spike answers the Phase 1 questions in `docs/design/wasm-grammars.md` using
TypeScript: can kawi load a tree-sitter grammar compiled to WebAssembly, parse
and highlight with it, and get the same result as the native grammar?

## Result

Yes, for TypeScript on this machine:

| Check | Result |
| --- | --- |
| Native and WASM parse trees | Identical across the corpus, a 190 KB generated file, and error input |
| Incremental reparse after insert, replace, and delete | Identical |
| Highlight spans with kawi's TypeScript queries | Identical |
| One loaded grammar shared by four threads | Works, one parser and store per thread |
| First load, release build | ~111 ms |
| Parse 190 KB, release build | ~107 ms WASM, ~61 ms native |
| Highlight 19 KB, release build | ~179 ms WASM, ~223 ms native |

The highlight numbers include query compilation because the test builds a new
`HighlightConfiguration` per call. Kawi compiles queries once per language, so
those numbers say more about setup than throughput. Debug builds are slower
(first load ~1.8 s, 190 KB parse ~1 s), which is why the CI budgets are 10 s.

## Layout

- `src/grammar.rs`: creates the Wasmtime engine, verifies and loads the module,
  and returns a `Grammar` that hands out parsers.
- `src/hash.rs`: SHA-256 verification before the module reaches Wasmtime.
- `src/abi.rs`: tree-sitter ABI range check.
- `src/error.rs`: load errors.
- `fixtures/typescript.wasm`: the TypeScript grammar, built from tree-sitter
  CLI 0.26.9 and wasi-sdk 29.0. Provenance is in `fixtures/PROVENANCE.md`.
- `tests/`: loader failures, parse parity, highlight parity, threading, and
  latency.

## Engine configuration

Bulk memory and reference types must stay on. Tree-sitter's Wasm stdlib uses
bulk memory operations, and wasi-sdk emits an overlong `call_indirect` encoding
that Wasmtime rejects with `zero byte expected` when reference types are off.

SIMD, relaxed SIMD, multi-memory, memory64, tail calls, function references,
GC, multi-value, branch hinting, extended const, and wide arithmetic are off.
A module that needs a disabled feature fails to load with `LoadError::Wasm`.

## Loading order

`Parser::set_wasm_store` must run before `set_language`. A `Language` belongs to
the `Engine` that produced it and can be shared by stores from that engine. A
parser without a store rejects the language with `cannot use wasm language
without a wasm store`.

## Wasmtime version conflict

`tree-sitter 0.27.2` with the `wasm` feature depends on
`wasmtime-c-api-impl 48.0.10`. Kawi pins `wasmtime 50.0.3`. The spike is a
separate workspace excluded from the root so the shipped build does not link
two Wasmtime versions. Before Phase 1, pick one of:

1. Align kawi to Wasmtime 48.x.
2. Wait for tree-sitter to move to 50.x.
3. Put the grammar runtime in a sidecar process.

This is the main open question.

## Running

```
cargo test --manifest-path spike/wasm-grammar/Cargo.toml
cargo test --manifest-path spike/wasm-grammar/Cargo.toml --release -- --nocapture
```

The second command prints the load, parse, and highlight timings. The CI
workflow `.github/workflows/wasm-grammar-spike.yml` runs format, clippy, fixture
validation, and the full test suite when the spike changes.

## Rebuilding the fixture

Check out tree-sitter-typescript at
`f975a621f4e7f532fe322e13c4f79495e0a7b2e7`, then run:

```
scripts/build-fixture.sh /path/to/tree-sitter-typescript
```

The script builds the `typescript/` grammar, not `tsx/`, and prints the new
SHA-256. Update `fixtures/PROVENANCE.md` and the constant in
`tests/support/mod.rs` when the hash changes.

## Open questions

- Wasmtime version alignment, as above.
- Pack discovery and caching: this spike loads a path known at compile time.
- Query ownership: the tests borrow kawi's `.scm` files. Packs need to carry
  their own queries or map to a query set by language.
- Sandbox limits: the engine currently enables only the required features. If
  grammar packs can come from outside kawi, add fuel or epoch limits and test
  a malicious module.
- Cold start at scale: one 1.4 MB module costs ~111 ms. A pack set needs its
  own measurement.
- Per-parser memory: not measured. Each parser creates a Wasm store, so the
  cost of many open files should be checked before Phase 1 ships.

## Recommendation

Proceed with WebAssembly grammar packs. Tree, incremental parse, and highlight
parity all hold, and sharing one loaded language across threads works. Resolve
the Wasmtime version question before wiring the loader into kawi.
