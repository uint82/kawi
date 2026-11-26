# TypeScript grammar fixture

- grammar: tree-sitter-typescript
- crate version: 0.23.2
- upstream revision: f975a621f4e7f532fe322e13c4f79495e0a7b2e7
- source: https://github.com/tree-sitter/tree-sitter-typescript
- tree-sitter CLI: 0.26.9
- wasi-sdk: 29.0
- sha256: 13677574a6375e1b12e98579850c55a4db5952ad8bacd673110007ac7cfd7e69

The module exports `tree_sitter_typescript` and imports `iswspace`, `iswalpha`,
the dylink memory, table, and base globals from the tree-sitter Wasm stdlib.

Rebuild with:

```
scripts/build-fixture.sh /path/to/tree-sitter-typescript
```

The checkout must be at the upstream revision above. The script builds the
`typescript/` grammar, not `tsx/`.
