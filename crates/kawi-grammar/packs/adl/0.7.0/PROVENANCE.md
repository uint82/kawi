# ADL grammar pack

- Upstream: https://github.com/alexytsu/tree-sitter-adl
- Revision: `5ffcda9174faa0b24331513c00f35cac823c1960`
- Crate: `tree-sitter-adl 0.7.0` (crates.io)
- License: MIT (see `LICENSE`)
- ABI: 15 (`parser.c` declares `LANGUAGE_VERSION 15`)
- Query: `queries/highlights.scm` copied unchanged from the crate

## Build

`grammar.wasm` was built from the crate source with:

```
tree-sitter build --wasm
```

using tree-sitter 0.26.9 and wasi-sdk 29.0. The result was checked with
`wasm-tools validate`.

```
sha256  3ffec313c96b3f2b78a408718403f0e8fb23570c98befb78ee849d9b37093a18
size    28500 bytes
```

## Determinism

The wasm build is not byte-for-byte reproducible: a second build of the same
source produced 28501 bytes with a different digest. CI must rebuild and
validate the module, but must not compare a rebuild against this fixture.
