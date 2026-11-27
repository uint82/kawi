# WebAssembly grammar packs

Status: proposed. WASM grammar packs are the primary extensible grammar system. A small native static core remains for bootstrap and fallback. Dynamic native grammar loading is out of scope for this plan.

## Decision

Kawi currently links every tree-sitter grammar into `kawi-syntax`. That works for a curated set, but it does not scale to an extensible language ecosystem. Every new language becomes a new dependency, a new build input, and a new release requirement.

The target model is:

- A small native core that works without installation or network access.
- WASM grammar packs for everything else.
- One platform-independent grammar artifact per language.
- Lazy loading, strict verification, and a clear fallback.
- Language configuration kept separate from grammar code.

Native dynamic loading is a possible future path for trusted system grammars, but it is not part of this plan. Maintaining native and WASM pack systems in parallel would double the packaging, signing, ABI, and testing work before the WASM path is proven.

## Goals

- Add a language without rebuilding the editor.
- Keep the editor download and install size bounded.
- Run third-party parsers inside a sandbox.
- Produce one grammar artifact for Linux, macOS, and Windows.
- Preserve current highlighting behavior and latency.
- Make grammar installation deterministic and verifiable.
- Keep a useful editor when a grammar pack is missing or broken.

## Non-goals

- Installing language servers or formatters.
- Replacing the language configuration in `kawi-lsp`.
- Hot-swapping a grammar while buffers using it are open.
- Auto-loading grammars from project directories.
- A full native dynamic loader in the first version.
- A public grammar registry in the first version.

## Terms

- Grammar: the tree-sitter parser generated from a grammar repository.
- Grammar pack: a versioned directory or archive containing a grammar module, queries, and a manifest.
- Grammar id: stable identifier for the parser, for example `adl` or `markdown_inline`.
- Language id: identifier used by the editor and language configuration, for example `adl`.
- Core grammar: a grammar linked into the native binary.
- External grammar: a grammar loaded from a pack.

## Current state

- `crates/kawi-syntax/Cargo.toml` lists every grammar crate as a direct dependency.
- `crates/kawi-syntax/src/language/grammar.rs:5` maps `LanguageId` to a `tree_sitter::Language` at compile time.
- `crates/kawi-syntax/src/language.rs` owns the closed `LanguageId` enum and the query cache.
- Queries are crate constants or `include_str!` files.
- User query overrides already exist under the query directory.
- `crates/kawi-lsp/src/defaults/languages` owns comments, indentation, pairs, root markers, and server defaults.
- `crates/kawi-plugin` uses Wasmtime 50 with the component model for plugins.

The current model is safe and fast because parser, queries, and host are built together. The plan must preserve those properties for the core while adding a sandboxed extension path.

## Architecture

Use tree-sitter's built-in `wasm` feature and `WasmStore`. Do not build a custom tree-sitter Wasm host.

The load path is:

1. Read and verify the grammar pack.
2. Check the pack format, grammar ABI, and kawi compatibility.
3. Load the Wasm module through `WasmStore`.
4. Cache the resulting `Language`.
5. Compile highlight and injection queries natively.
6. Attach a `WasmStore` to each parser that needs it.
7. Parse and highlight as usual.

Tree-sitter's Wasm path is hybrid. The grammar's static parse tables are copied into native memory. Only the lexer and external scanner run in Wasm. The syntax tree stays native. This avoids copying trees across the Wasm boundary on every edit and keeps performance close to native.

Grammar Wasm modules are core WebAssembly modules using tree-sitter's ABI. They are not WASI components. Kawi's component-model plugin runtime is a separate system.

### Crate boundaries

- `kawi-grammar`: pack discovery, manifests, hashes, installation, compatibility checks, and Wasm loading.
- `kawi-syntax`: parsers, queries, highlighting, injections, and trees.
- `kawi-lsp`: language configuration and server defaults.
- `kawi`: install commands, status reporting, and user experience.

`kawi-grammar` should not depend on editor state. `kawi-syntax` should not know where a pack came from. `kawi-lsp` should not load grammars.

### Language identity

Keep `LanguageId` for built-in languages. Do not expand it for every external pack.

Add a `GrammarId` string for pack identity and a language registry that maps file types and aliases to a language. Built-in language configuration remains authoritative. A pack may supply file types for a language that has no built-in configuration.

External languages must be able to exist without adding a Rust enum variant.

### Parser pool

Follow the model used by Zed:

- One Wasmtime engine per process, created once.
- A bounded pool of reusable parsers.
- Each parser owns a `WasmStore`.
- Each grammar is loaded once through a temporary parser store, then the resulting `Language` is cached.
- Parsers can use a language loaded from another store when the engine is the same.
- Do not share a mutable store across threads.

The engine and stores are process infrastructure. The language and query caches belong to the syntax layer.

## Pack format

A pack contains:

- `grammar.wasm`
- `queries/highlights.scm`
- `queries/injections.scm` when needed
- `manifest.toml`
- upstream license and provenance

The manifest contains:

- pack format version
- grammar id and language id
- grammar revision
- tree-sitter ABI minimum and maximum
- Wasm SHA-256
- query file hashes
- file types and aliases
- minimum kawi version
- upstream repository and revision
- license
- optional signature

Grammar and queries are versioned together. A grammar update can rename nodes and invalidate queries. A pack must never mix a new grammar with old queries.

Do not put comments, indentation, auto pairs, root markers, or LSP server configuration in the grammar pack. Those belong in language configuration. A pack may provide defaults for a new language, but the editor configuration must be able to override them.

## Build pipeline

Build grammar packs in CI. Do not compile Wasm on the user's machine.

Pin:

- tree-sitter CLI version
- WASI SDK version
- grammar repository revision
- compiler flags

For each grammar:

1. Check out the pinned revision.
2. Run the upstream tree-sitter tests.
3. Build the Wasm module with `tree-sitter build --wasm`.
4. Validate it with `wasm-tools validate`.
5. Inspect imports and exports.
6. Run the kawi parity tests.
7. Record hashes and provenance.
8. Sign or checksum the final pack.

Wasm artifacts are platform-independent. Build once and publish for all supported platforms. This is the main packaging advantage over native grammar libraries.

Builds should be reproducible. Rebuilding the same revision with the same toolchain should produce the same module hash. If that is not possible, record the difference and avoid claiming reproducibility.

## Installation

Use versioned directories:

- bundled packs: read-only, shipped with kawi
- user packs: `$XDG_DATA_HOME/kawi/grammars/<id>/<version>/`
- cache: `$XDG_CACHE_HOME/kawi/grammars/`
- system packs: read-only, for distributions

Install to a temporary directory, verify the pack, then rename it into place. Never leave a partially installed pack visible.

Keep the previous version until the new version is verified. Allow the user to pin a version and roll back.

Do not auto-load grammars from a project directory. A repository could otherwise ship a parser and get it executed. Project-local grammars require explicit trust and should not be part of the first version.

## Runtime

Load packs lazily when a buffer with a matching language opens. Do not instantiate every grammar at startup.

Verify the pack hash before loading. Verify the tree-sitter ABI before using the language. Compile queries and reject a pack if the queries do not match the grammar.

Use a single engine and cache compiled modules where Wasmtime supports it. Cache loaded languages and compiled queries in memory.

If a pack is missing, corrupt, incompatible, or fails to load:

- keep the buffer editable
- fall back to the native core grammar if one exists
- otherwise fall back to plain text
- report the failure through `:health syntax` or `:grammar status`

A grammar failure must never crash the editor.

## Resource limits

Tree-sitter's `WasmStore` caps Wasm memory at 128 MB. Treat that as a safety net, not a tuning target.

Configure the Wasmtime engine for the smallest feature set that works. Do not enable WASI, filesystem access, networking, or host functions that a grammar does not need.

There is an open gap around CPU limits. `WasmStore` does not expose per-parse fuel or epoch deadlines. A malicious or broken external scanner can still spin and consume CPU. If hard CPU isolation is required, add a watchdog or an upstream API change before claiming full sandbox safety.

Queries run natively, not inside Wasm. Validate query size and handle query compilation errors. Do not treat a grammar pack as a way to run arbitrary query code.

## Security

WASM prevents native code execution and process corruption from grammar code. It does not make a grammar trustworthy.

- Only load packs from trusted locations.
- Verify checksums before every load, not only at install time.
- Sign official packs if they are distributed over the network.
- Pin grammar revisions and toolchain versions.
- Reject unexpected imports and exports.
- Do not auto-load project-local packs.
- Treat native core grammars as trusted code.
- Keep Wasmtime and tree-sitter current for security fixes.

Third-party grammar packs are code. They can produce incorrect trees, consume CPU, and trigger parser bugs. The sandbox limits the damage, but it does not eliminate it.

## Compatibility

A pack declares the tree-sitter ABI range it supports. The loader rejects packs outside the host range with an actionable error.

Queries are tied to a grammar revision. A pack update must update grammar and queries together.

A pack declares the minimum kawi version. Kawi should reject packs that require a newer host and warn when a pack targets an older, unsupported format.

The Wasmtime dependency is resolved. Kawi aligns to 48.0.10, the version tree-sitter 0.27.2 pins. Wasmtime 48 is an LTS release supported into 2028, while the previous 50.0.3 was a normal release with a short support window. One engine keeps binary size and the security surface down.

The alternatives were considered and rejected: patching tree-sitter to use a newer Wasmtime adds maintenance and upstream risk, and accepting two engines adds binary size and a second vulnerability response path for no feature gain.

## Testing

Unit tests:

- manifest parsing and validation
- hash mismatch
- missing files
- ABI mismatch
- invalid query
- unexpected imports
- path traversal and symlink cases

Integration tests:

- load a pack and parse a file
- highlight spans match the native grammar
- incremental edits produce the same tree
- injections work
- multiple buffers share cached languages
- parser pool reuse works
- missing pack falls back cleanly

Parity tests:

- compare native and Wasm syntax trees for a corpus
- compare highlight spans
- compare incremental reparse results
- compare indentation and injection behavior

Failure tests:

- corrupt Wasm
- wrong ABI
- missing export
- trapped scanner
- memory limit
- infinite scanner loop, with the documented timeout behavior
- pack removed while loaded
- duplicate grammar ids

Performance tests:

- first load time
- parse latency on small and large files
- highlight latency
- memory use
- startup with many installed packs

Fuzz tests:

- manifest input
- Wasm loader
- query text
- source input

Cross-platform tests:

- Linux, macOS, and Windows hosts
- one Wasm artifact on every host
- macOS and Windows loader behavior
- path and permission failures

## Migration

### Phase 0: Decisions

- Confirm WASM as the primary extensible path.
- Confirm the native static core scope.
- Resolve the Wasmtime version conflict.
- Decide whether external languages use a string registry in the first version.
- Decide whether packs ship with releases, download on demand, or both.

### Phase 1: Spike

Use ADL as the first grammar. It is small, new, and already covered by highlight tests. Then test JavaScript or another grammar with an external scanner.

Exit criteria:

- native and Wasm trees match
- native and Wasm highlights match
- incremental parsing works
- parser pool and threading work
- failure modes are contained
- latency is acceptable

### Phase 2: Infrastructure

- Add the pack format and manifest.
- Add the build pipeline.
- Add the loader and parser integration behind a feature flag.
- Add install, status, and fallback behavior.
- Add parity tests.

### Phase 3: Built-in migration

Move non-core built-in grammars to packs one at a time. Keep the native core during the migration. Remove a static dependency only after its Wasm pack passes parity tests.

Start with ADL, then a scanner grammar, then injection-heavy grammars such as Markdown and HTML.

ADL ships as a bundled pack in release archives. The native ADL grammar remains as fallback until the pack passes parity tests on every release target.

### Phase 4: External languages

Add the string-based language registry. Allow third-party packs only from explicit installs. Add provenance, signatures, and update policy.

### Phase 5: Cleanup

Remove unused static grammar dependencies. Keep the small native core. Update documentation and packaging.

## Rollout

Use a separate feature flag from the plugin runtime. A user should be able to enable WASM grammars without enabling plugins, and the other way around.

Dogfood the pack system with ADL and a few core languages before making it the default. Keep the native static core as the fallback for at least one release cycle.

Document:

- how to install a grammar pack
- how to pin or roll back a version
- how to inspect loaded grammars
- what a failed pack means
- the trust model for third-party packs

## Industry standard

Zed uses Wasmtime and tree-sitter's `WasmStore` for language extensions. Extension archives contain a grammar Wasm module, queries, and an extension component. Zed compiles grammars to Wasm with a pinned WASI SDK and runs them in Wasmtime. This is the closest match for kawi.

VS Code uses web-tree-sitter and Wasm parsers for its extension model.

Helix and Neovim use dynamically loaded native parsers built from pinned grammar sources. That model is faster and simpler at runtime, but it requires a C toolchain and trusts grammar code with the editor process.

Tree-sitter's own CLI builds native shared libraries and Wasm modules. Its `WasmStore` is the supported Rust API for loading Wasm grammars.

The common pattern is clear: editors do not statically link every grammar once they support many languages. They use native dynamic parsers or Wasm parsers. Kawi is choosing the Wasm path for portability and isolation.

## Open questions

- Which Wasmtime version wins, and what changes for the plugin runtime?
- How are CPU limits enforced for a spinning scanner?
- Does the native core stay permanently, or only for bootstrap?
- When should project-local grammar packs be allowed, if ever?
- What signature scheme is used for official packs?
- Are packs bundled with releases or downloaded on demand? ADL is bundled; larger grammars may need a different answer.
- How are grammar pack updates rolled out and rolled back?
- How are user query overrides handled for external languages?
- Does a pack supply language configuration defaults, or only parsing and queries?

## References

- tree-sitter Wasm grammar API: https://docs.rs/tree-sitter/latest/tree_sitter/
- tree-sitter build command: https://tree-sitter.github.io/tree-sitter/cli/build.html
- Zed language extensions: https://zed.dev/blog/language-extensions-part-1
- Zed extension archive: https://zed.dev/blog/zed-decoded-extensions
- Helix languages configuration: https://docs.helix-editor.com/languages.html
- Existing kawi WASM plugin design: `docs/design/plugin-wasm.md`
