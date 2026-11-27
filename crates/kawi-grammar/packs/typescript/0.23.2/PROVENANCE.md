# TypeScript grammar pack

- Upstream: https://github.com/tree-sitter/tree-sitter-typescript
- Revision: `f975a621f4e7f532fe322e13c4f79495e0a7b2e7`
- Crate: `tree-sitter-typescript 0.23.2` (crates.io)
- License: MIT (see `LICENSE`)
- ABI: 14 (`typescript/src/parser.c` declares `LANGUAGE_VERSION 14`)
- Scanner: `typescript/src/scanner.c` is compiled into `grammar.wasm`
- Highlights: `queries/highlights.scm` is the exact concatenation of
  `ecmascript.scm`, `ecmascript-tokens.scm`, and `typescript.scm` from
  `crates/kawi-syntax/queries`
- Locals: `queries/locals.scm` copied unchanged from the crate

## Build

`grammar.wasm` was built from the crate source with:

```
tree-sitter build --wasm
```

using tree-sitter 0.26.9 and wasi-sdk 29.0. The result was checked with
`wasm-tools validate`.

```
sha256  36b57857943c4f7a26d877e5588c917a5f0c6eb103a7cdebd1fe146b0c78d7f1
size    1418209 bytes
```

## Determinism

The wasm build is not byte-for-byte reproducible. CI must rebuild and validate
the module, but must not compare a rebuild against this fixture.
