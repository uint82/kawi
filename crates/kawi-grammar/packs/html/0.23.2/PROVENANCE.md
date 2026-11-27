# HTML grammar pack

- Upstream: https://github.com/tree-sitter/tree-sitter-html
- Revision: `5a5ca8551a179998360b4a4ca2c0f366a35acc03` (`v0.23.2`)
- Crate: `tree-sitter-html 0.23.2` (crates.io)
- License: MIT (see `LICENSE`)
- ABI: 14 (`src/parser.c` declares `LANGUAGE_VERSION 14`)
- Scanner: `src/scanner.c` and `src/tag.h` are compiled into `grammar.wasm`
- Highlights: `queries/highlights.scm` copied unchanged from
  `crates/kawi-syntax/queries/html.scm`
- Injections: `queries/injections.scm` copied unchanged from the crate

## Build

`grammar.wasm` was built from the crate source with:

```
tree-sitter build --wasm
```

using tree-sitter 0.26.9 and wasi-sdk 29.0. The result was checked with
`wasm-tools validate`.

```
sha256  91941c44a17a53939c79532214531877c816fa6d7397564d88ce34612d4d25d6
size    20412 bytes
gzip    7252 bytes
xz      5800 bytes
```

## Determinism

The wasm build is not byte-for-byte reproducible. CI must rebuild and validate
the module, but must not compare a rebuild against this fixture.
