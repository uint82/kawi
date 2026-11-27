# Grammar packs

Kawi can load tree-sitter grammars from WebAssembly packs. Release binaries include the grammar runtime and the bundled ADL pack. Source builds opt in with `--features wasm-grammars`:

```sh
cargo install --path crates/kawi --features wasm-grammars --locked
```

Cargo installs do not include the bundled pack. Install a pack explicitly when you use a source build. The grammar feature is independent of `--features wasm`; enable either, both, or neither.

## Install

```sh
kawi grammar install /path/to/pack
kawi grammar status
```

Packs install to `$XDG_DATA_HOME/kawi/grammars/<id>/<version>/`. Installation copies the pack into a temporary directory, verifies every hash, and renames it into place, so a failed install never leaves a partial pack visible.

User packs take precedence over bundled packs. Within one root, Kawi picks the highest valid version.

`kawi grammar status` lists each discovered pack with its root, state, and path:

```
adl 0.7.0 [user] valid /home/me/.local/share/kawi/grammars/adl/0.7.0
adl 0.7.0 [bundled] valid /opt/kawi/share/kawi/grammars/adl/0.7.0
```

States are `valid`, `unverified`, `invalid`, and `unloadable`. An `invalid` pack failed verification, such as a hash mismatch or an ABI range that does not overlap this build. An `unloadable` pack passed verification but failed the editor version check, the grammar ABI check, query compilation, or targeted a language this build does not know.

## Pack layout

A pack contains:

```
manifest.toml
grammar.wasm
queries/highlights.scm
queries/injections.scm   optional
queries/locals.scm       optional
LICENSE
PROVENANCE.md
```

`manifest.toml` records the pack format, grammar and language ids, version, grammar revision, ABI range, hashes, minimum kawi version, repository, and license. Grammar and queries are versioned together; do not mix a new grammar with old queries.

Queries must use capture names kawi already understands, such as `@comment`, `@keyword`, `@string`, and `@function`. A pack cannot add new capture names yet.

## Bundled packs

Release archives put the bundled pack under `share/kawi/grammars` beside the `kawi` binary. Kawi also checks `share/kawi/grammars` one directory above the binary, which covers installs that keep the binary in `bin/`.

Bundled packs are read-only. To override one, install a user pack for the same id. The user pack wins even when the bundled pack has a newer version.

## Runtime behavior

- At startup kawi discovers user and bundled packs, verifies their hashes, and registers the best valid version for each built-in language.
- The grammar loads lazily when a buffer with that language opens.
- If a pack fails to load, kawi keeps the native grammar and queries for that language and reports the failure.
- This phase accepts built-in language ids only. A pack cannot introduce a new language name.

## Trust model

Grammar packs run as WebAssembly inside the editor process. The Wasm engine restricts the module to the operations tree-sitter needs, but a parser is still code. Install only packs you trust.

Kawi does not load project-local packs, and this phase has no signatures or update channel. Treat a pack like any other executable dependency.

## Build a pack

```sh
scripts/build-grammar-pack.sh <grammar-source-dir> <pack-dir>
```

The script requires tree-sitter CLI 0.26.9, wasm-tools, and wasi-sdk 29.0. It builds `grammar.wasm`, validates the module, and prints the SHA-256 and size. Update `manifest.toml` and `PROVENANCE.md` with the result.

Wasm builds are not byte-for-byte reproducible. The committed hash identifies the committed artifact; CI rebuilds and validates the module but does not compare a rebuild to the committed hash.

## Limitations

- System pack roots are not loaded yet. Bundled packs are loaded from the release layout described above.
- There is no pin or rollback command. Previous versions remain on disk, and the user root overrides bundled and system roots.
- There is no external language registry.
- There are no pack signatures.
