# WebAssembly grammar packs

Status: proposed, strategy selected for review. WASM grammar packs are the only runtime grammar system in the target state. Releases bundle a small Wasm baseline and install other languages on demand from the official `kawi-grammar` source. Native grammar crates are test-only. Dynamic native grammar loading is out of scope for this plan.

## Decision

Kawi currently links every tree-sitter grammar into `kawi-syntax`. That works for a curated set, but it does not scale to an extensible language ecosystem. Every new language becomes a new dependency, a new build input, and a new release requirement. The native release build also pays the compile and link cost of every grammar crate.

The target model is:

- No native runtime grammar dependencies. The release binary links no tree-sitter grammar crates.
- A small bundled Wasm baseline so a fresh install works offline.
- Official Wasm packs for every other language, installed on demand.
- One platform-independent grammar artifact per language.
- Lazy loading, strict verification, and plain-text fallback.
- Language configuration kept separate from grammar code.
- One official `kawi-grammar` source, distributed through one versioned registry release with per-pack assets.

The purpose is to keep the native build lightweight. The release build should not compile or link 100+ tree-sitter grammar crates. Bundling Wasm packs affects download size, not native compile or link cost.

A separate grammar repository is not required. The official source can live in the Kawi project. If pack release cadence later justifies a separate repository, the client contract stays the same.

Native dynamic loading is a possible future path for trusted system grammars, but it is not part of this plan. Maintaining native and WASM pack systems in parallel would double the packaging, signing, ABI, and testing work before the WASM path is proven.

## Goals

- Add a language without rebuilding the editor.
- Keep the native release build free of grammar crates.
- Keep the editor download and install size bounded.
- Ship a small offline baseline for first run.
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
- A public third-party grammar registry in the first version.
- Bundling every available language in the release archive.

## Terms

- Grammar: the tree-sitter parser generated from a grammar repository.
- Grammar pack: a versioned directory or archive containing a grammar module, queries, and a manifest.
- Grammar id: stable identifier for the parser, for example `adl` or `markdown_inline`.
- Language id: identifier used by the editor and language configuration, for example `adl`.
- Alias: user-facing name accepted by commands, for example `markdown`, `md`, `c++`, or `csharp`.
- Baseline pack: a Wasm pack shipped with kawi so first run works offline.
- Available pack: a pack published in the registry but not bundled.
- Native test grammar: a native grammar crate used only by parity tests and never shipped.
- Pack grammar: a grammar loaded from a Wasm pack.

## Current state

- `crates/kawi-syntax/Cargo.toml` lists every grammar crate as a direct dependency.
- `crates/kawi-syntax/src/language/grammar.rs:5` maps `LanguageId` to a `tree_sitter::Language` at compile time.
- `crates/kawi-syntax/src/language.rs` owns the closed `LanguageId` enum and the query cache.
- Queries are crate constants or `include_str!` files.
- User query overrides already exist under the query directory.
- `crates/kawi-lsp/src/defaults/languages` owns comments, indentation, pairs, root markers, and server defaults.
- `crates/kawi-plugin` uses Wasmtime 48.0.10 with the component model for plugins.

The current model is safe and fast because parser, queries, and host are built together. The plan must preserve those properties for baseline languages while moving parser execution to Wasm and removing grammar crates from the native release build.

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

Keep `LanguageId` for built-in language configuration. Do not expand it for every external pack.

Add a `GrammarId` string for pack identity and a language registry that maps file types and aliases to a language. Built-in language configuration remains authoritative. A pack may supply file types for a language that has no built-in configuration.

External languages must be able to exist without adding a Rust enum variant.

Use stable `GrammarId` values as the canonical key for manifests, indexes, caches, hashes, and installed directories. Accept language names and aliases at the command boundary and resolve them to a `GrammarId`. Do not use a display name as a primary key.

Names and grammar identities are not one-to-one. `markdown` and `markdown_inline` are separate grammars, while `c++`, `cpp`, and `cxx` may all refer to one grammar. Alias resolution must reject unknown and ambiguous names with an actionable error.

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
- pack version
- grammar id and language id
- display name and command aliases
- grammar revision
- tree-sitter ABI minimum and maximum
- Wasm SHA-256
- query file hashes
- file extensions and language aliases
- minimum kawi version
- upstream repository and revision
- license
- optional signature

Grammar and queries are versioned together. A grammar update can rename nodes and invalidate queries. A pack must never mix a new grammar with old queries.

Do not put comments, indentation, auto pairs, root markers, or LSP server configuration in the grammar pack. Those belong in language configuration. A pack may provide defaults for a new language, but the editor configuration must be able to override them.

### Registry and distribution

Use one official `kawi-grammar` source and one versioned registry release per pack-set snapshot. A release contains one small `index.json` and one artifact per pack. Do not create one release per grammar. Published registry releases are immutable; fixes use a new release.

The index contains:

- registry format version
- grammar id, display name, and aliases
- pack version
- download URL
- SHA-256 and size
- license
- minimum kawi version
- tree-sitter ABI range
- baseline flag
- upstream repository and revision

A release may also publish an optional full bundle for mirrors and offline installs. It is not the default download.

The baseline is a small subset that ships inside the Kawi release archive. Every other language is available from the registry but is not bundled. This keeps the Kawi download bounded while still supporting 100+ languages.

The expected baseline is the set needed to edit Kawi itself: Rust, TOML, Markdown, and Bash. The exact set remains open until release packaging is finalized.

Resolution order for an install:

1. Look up the requested name or alias in the cached index.
2. Select the newest pack compatible with the host kawi version and tree-sitter ABI.
3. Download only that pack.
4. Verify its hash, manifest, and compatibility.
5. Install it atomically.

The client must never fetch an arbitrary repository or moving branch. It downloads immutable artifacts from the official registry. If the registry moves to object storage later, only the index URLs change.

## Build pipeline

Build grammar packs in CI. Do not compile Wasm on the user's machine.

The Kawi release build must not compile or link native grammar crates. Native grammar crates are test-only dependencies used as the parity oracle. A release check should fail if a grammar crate appears in the shipped dependency graph.

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
6. Run the kawi parity tests against the native test grammar.
7. Record hashes and provenance.
8. Sign or checksum the final pack.

The pipeline also generates `index.json`, builds the baseline archive, and verifies that every baseline pack is present and loadable in the release archive. It publishes one registry release with one asset per pack and publishes the baseline with the Kawi release.

Wasm artifacts are platform-independent. Build once and publish for all supported platforms. This is the main packaging advantage over native grammar libraries.

Builds should be reproducible. Rebuilding the same revision with the same toolchain should produce the same module hash. If that is not possible, record the difference and avoid claiming reproducibility.

## Installation

Use versioned directories:

- baseline packs: read-only, shipped with kawi
- user packs: `$XDG_DATA_HOME/kawi/grammars/<id>/<version>/`
- cache: `$XDG_CACHE_HOME/kawi/grammars/`
- system packs: read-only, for distributions

Install to a temporary directory, verify the pack, then rename it into place. Never leave a partially installed pack visible.

Keep the previous version until the new version is verified. Allow the user to pin a version and roll back.

`:grammar install <name>` accepts a language name or alias, resolves it through the cached index to a stable `GrammarId`, and installs the newest compatible pack from the official registry. The command must not accept an arbitrary repository or branch.

When a buffer opens with no matching pack, prompt the user to install it. Do not install silently. The prompt names the language, the pack version, the download size, and the source.

All official packs come from the `kawi-grammar` registry. A separate repository is not required. The registry is one release with many pack assets, not one release per grammar.

Do not auto-load grammars from a project directory. A repository could otherwise ship a parser and get it executed. Project-local grammars require explicit trust and should not be part of the first version.

## Runtime

Load packs lazily when a buffer with a matching language opens. Do not instantiate every grammar at startup.

Verify the pack hash before loading. Verify the tree-sitter ABI before using the language. Compile queries and reject a pack if the queries do not match the grammar.

Use a single engine and cache compiled modules where Wasmtime supports it. Cache loaded languages and compiled queries in memory.

If a pack is missing, corrupt, incompatible, or fails to load:

- keep the buffer editable
- load the matching baseline pack if the language is part of the baseline
- otherwise fall back to plain text
- report the failure through `:health syntax` or `:grammar status`

There is no native parser fallback in the target state. During migration, the interim native fallback remains as described in Migration. A grammar failure must never crash the editor.

## Resource limits

Tree-sitter's `WasmStore` caps Wasm memory at 128 MB. Treat that as a safety net, not a tuning target.

Configure the Wasmtime engine for the smallest feature set that works. Do not enable WASI, filesystem access, networking, or host functions that a grammar does not need.

There is an open gap around CPU limits. `WasmStore` does not expose per-parse fuel or epoch deadlines. A malicious or broken external scanner can still spin and consume CPU. If hard CPU isolation is required, add a watchdog or an upstream API change before claiming full sandbox safety.

Queries run natively, not inside Wasm. Validate query size and handle query compilation errors. Do not treat a grammar pack as a way to run arbitrary query code.

## Security

WASM prevents native code execution and process corruption from grammar code. It does not make a grammar trustworthy.

- Only load packs from the official registry, explicit user installs, and trusted system locations.
- Verify checksums before every load, not only at install time.
- Sign the official registry index and packs before on-demand installation becomes the default.
- Pin grammar revisions and toolchain versions.
- Reject unexpected imports and exports.
- Do not install from arbitrary GitHub URLs or moving branches.
- Do not auto-load project-local packs.
- Treat native test grammars as trusted test code that must never be shipped.
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
- resolve a language name and alias to the correct `GrammarId`
- reject unknown and ambiguous names
- parse and filter the registry index by kawi version and tree-sitter ABI
- download and verify one pack asset
- highlight spans match the native test grammar
- incremental edits produce the same tree
- injections work
- multiple buffers share cached languages
- parser pool reuse works
- missing pack falls back cleanly

Parity tests:

- compare native test grammar and Wasm syntax trees for a corpus
- compare highlight spans
- compare incremental reparse results
- compare indentation and injection behavior

Release tests:

- every baseline pack is present in the release archive
- every baseline pack loads on every release target
- no native grammar crate appears in the release dependency graph
- the registry release contains exactly one index and one asset per pack
- installing from the index fetches only the requested pack

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

Resolved:

- Wasm is the only runtime grammar path in the target state.
- There is no native grammar core in the target state. Native grammar crates remain test-only.
- The Wasmtime version conflict is resolved at 48.0.10.
- External languages use the string registry in Phase 4.
- Releases bundle a small Wasm baseline and install other languages on demand from the official registry.

### Phase 1: Spike

Use ADL as the first grammar. It is small, new, and already covered by highlight tests. Then test JavaScript or another grammar with an external scanner.

Exit criteria:

- native test grammar and Wasm trees match
- native test grammar and Wasm highlights match
- incremental parsing works
- parser pool and threading work
- failure modes are contained
- latency is acceptable

### Phase 2: Infrastructure

- Add the pack format and manifest.
- Add the registry index and release layout.
- Add the build pipeline.
- Add the loader and parser integration behind a feature flag.
- Add install, status, and fallback behavior.
- Add alias resolution and parity tests.

### Phase 3: Built-in migration

Move built-in grammars to packs one at a time. Keep the native grammars during migration as a temporary fallback. Do not remove a static dependency until its pack passes parity on every release target and has shipped for one release cycle.

Start with ADL, then a scanner grammar, then injection-heavy grammars such as Markdown and HTML.

ADL, TypeScript, Markdown, and HTML are the first migrated packs. TypeScript is the first scanner grammar. Markdown is the first injection-heavy grammar and uses separate block and inline packs. HTML is the second and injects JavaScript and CSS from script and style elements.

Markdown is expected to be part of the baseline. The other migrated packs are available from the registry unless the final baseline includes them. The interim native grammars remain as fallback through Phase 4.

### Phase 4: External languages

Add the string-based language registry. Allow third-party packs only from explicit installs. Add provenance, signatures, and update policy.

Publish one official registry release with an index and per-pack assets. The install command resolves names and aliases through that index and downloads only the requested pack. It does not fetch arbitrary repositories or branches.

### Phase 5: Cleanup

Remove all native runtime grammar dependencies. Keep native grammar crates only as test-only dependencies. Update documentation and packaging.

The release build must contain no grammar crates. The final failure mode is plain text, not a native parser fallback.

## Rollout

Use a separate feature flag from the plugin runtime. A user should be able to enable WASM grammars without enabling plugins, and the other way around.

Dogfood the pack system with the baseline languages before making it the default. Keep the native grammars as an interim fallback for at least one release cycle after the baseline packs ship. Remove them in Phase 5 after the release-target and one-cycle gates pass.

Ship the baseline packs in the release archive so first run works offline. Install every other language on demand from the official registry.

Document:

- how to install a grammar pack
- how names and aliases resolve to grammar ids
- how to pin or roll back a version
- how to inspect loaded grammars
- what a failed pack means
- the trust model for third-party packs
- why there is no native grammar fallback

## Industry standard

Zed uses Wasmtime and tree-sitter's `WasmStore` for language extensions. Extension archives contain a grammar Wasm module, queries, and an extension component. Zed compiles grammars to Wasm with a pinned WASI SDK and runs them in Wasmtime. This is the closest match for kawi.

VS Code uses web-tree-sitter and Wasm parsers for its extension model.

Helix and Neovim use dynamically loaded native parsers built from pinned grammar sources. That model is faster and simpler at runtime, but it requires a C toolchain and trusts grammar code with the editor process.

Tree-sitter's own CLI builds native shared libraries and Wasm modules. Its `WasmStore` is the supported Rust API for loading Wasm grammars.

The common pattern is clear: editors do not statically link every grammar once they support many languages. They use native dynamic parsers or Wasm parsers. Kawi is choosing the all-Wasm path for portability, isolation, and a lightweight native build. Unlike Zed, Kawi will not keep native built-in parsers. Unlike Helix and Neovim, Kawi will not require a C toolchain or load native grammar libraries.

## Open questions

- Which languages form the final baseline? Rust, TOML, Markdown, and Bash are the current proposal.
- How are CPU limits enforced for a spinning scanner?
- When should project-local grammar packs be allowed, if ever?
- What signature scheme is used for the official registry and packs?
- Where are registry assets hosted if the asset count outgrows a single GitHub release?
- Does the registry stay in the Kawi repository or move to a dedicated repository later?
- How are grammar pack updates rolled out and rolled back?
- How are user query overrides handled for external languages?
- Does a pack supply language configuration defaults, or only parsing and queries?
- How is the no-native-grammar release check enforced in CI?

## References

- tree-sitter Wasm grammar API: https://docs.rs/tree-sitter/latest/tree_sitter/
- tree-sitter build command: https://tree-sitter.github.io/tree-sitter/cli/build.html
- Zed language extensions: https://zed.dev/blog/language-extensions-part-1
- Zed language support: https://zed.dev/docs/languages
- Zed extension archive: https://zed.dev/blog/zed-decoded-extensions
- Helix grammar loader: https://github.com/helix-editor/helix/blob/master/helix-loader/src/grammar.rs
- Helix languages configuration: https://docs.helix-editor.com/languages.html
- Existing kawi WASM plugin design: `docs/design/plugin-wasm.md`
