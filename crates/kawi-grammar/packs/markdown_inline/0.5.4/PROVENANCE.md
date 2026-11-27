# Markdown inline grammar pack

- Upstream: https://github.com/tree-sitter-grammars/tree-sitter-markdown
- Revision: `41ff24a6c775492c543b09228aeb98d1cd2b3a0f`
- Crate: `tree-sitter-md 0.5.4` (crates.io)
- License: MIT (see `LICENSE`)
- ABI: 15 (`tree-sitter-markdown-inline/src/parser.c` declares `LANGUAGE_VERSION 15`)
- Scanner: `tree-sitter-markdown-inline/src/scanner.c` is compiled into `grammar.wasm`
- Highlights: `queries/highlights.scm` copied unchanged from
  `crates/kawi-syntax/queries/markdown-inline.scm`
- Injections: `queries/injections.scm` copied unchanged from
  `crates/kawi-syntax/queries/markdown-inline-injections.scm`

## Build

`grammar.wasm` was built from `tree-sitter-markdown-inline` with:

```
tree-sitter build --wasm
```

using tree-sitter 0.26.9 and wasi-sdk 29.0. The result was checked with
`wasm-tools validate`.

```
sha256  4426bd0aa98c7effbf21c12f4c2cb86fb03e9ef3931d1f6d8782f55d60fe7d63
size    426105 bytes
gzip    63462 bytes
xz      37684 bytes
```

## Determinism

The wasm build is not byte-for-byte reproducible. CI must rebuild and validate
the module, but must not compare a rebuild against this fixture.
