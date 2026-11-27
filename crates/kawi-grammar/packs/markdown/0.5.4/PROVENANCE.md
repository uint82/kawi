# Markdown grammar pack

- Upstream: https://github.com/tree-sitter-grammars/tree-sitter-markdown
- Revision: `41ff24a6c775492c543b09228aeb98d1cd2b3a0f`
- Crate: `tree-sitter-md 0.5.4` (crates.io)
- License: MIT (see `LICENSE`)
- ABI: 15 (`tree-sitter-markdown/src/parser.c` declares `LANGUAGE_VERSION 15`)
- Scanner: `tree-sitter-markdown/src/scanner.c` is compiled into `grammar.wasm`
- Highlights: `queries/highlights.scm` copied unchanged from
  `crates/kawi-syntax/queries/markdown.scm`
- Injections: `queries/injections.scm` copied unchanged from
  `crates/kawi-syntax/queries/markdown-injections.scm`

## Build

`grammar.wasm` was built from `tree-sitter-markdown` with:

```
tree-sitter build --wasm
```

using tree-sitter 0.26.9 and wasi-sdk 29.0. The result was checked with
`wasm-tools validate`.

```
sha256  474ad6d0f6e66b641c89d39a9f8afeb64f67be7a3a79679d82168e57a806c1c1
size    429752 bytes
gzip    62636 bytes
xz      38276 bytes
```

## Determinism

The wasm build is not byte-for-byte reproducible. CI must rebuild and validate
the module, but must not compare a rebuild against this fixture.
