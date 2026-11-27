# Grammar baseline

Status: decided. The permanent Wasm grammar baseline is locked. The registry and manifest contract is separate and still open.

## Decision

The baseline languages are Rust, TOML, Markdown, and Bash.

The release archive ships six baseline packs:

| Pack | Role | Why it is baseline |
| --- | --- | --- |
| `rust` | language | Kawi is written in Rust. |
| `toml` | language | Cargo manifests and Kawi configuration are TOML. |
| `markdown` | language | Documentation and help pages are Markdown. |
| `markdown_inline` | support | The Markdown block grammar injects inline spans into it. |
| `bash` | language | Build, CI, and install scripts are shell. |
| `yaml` | support | Markdown frontmatter and help page metadata are YAML. |

`markdown_inline` and `yaml` are supporting packs. They do not add baseline languages. The baseline language list remains Rust, TOML, Markdown, and Bash.

## Rationale

- The four languages cover the files needed to edit Kawi itself.
- Markdown is not complete without `markdown_inline`; the block grammar delegates inline spans to it.
- Kawi's help pages use YAML frontmatter. Without the YAML pack, that frontmatter becomes plain text after the native fallback is removed.
- TOML frontmatter is already covered by the TOML pack.
- Fenced code blocks can inject any installed language. That is optional and does not expand the baseline.

## Transitional bundles

ADL, TypeScript, and HTML are not baseline. They stay bundled for now so current behavior does not regress while the registry is built.

- ADL is standalone and can move to on-demand first.
- TypeScript covers the `typescript` grammar only, not `tsx`, `jsx`, or `javascript`.
- HTML injects JavaScript and CSS for `script` and `style` elements.

Bundling these three is a transition, not a permanent guarantee. Remove them from the release archive after the registry works and the packs they depend on are available on demand. Keep the native fallback per language until its pack passes parity on every release target and has shipped for one release cycle.

## Rules

- Baseline packs ship in every release archive and must load offline on every release target.
- Baseline packs are read-only. A user pack may override one by grammar id.
- The baseline is the only set bundled in the target state.
- A language cannot lose its native fallback until its baseline pack is present and verified.
- Markdown's baseline guarantees cover frontmatter through YAML and TOML, and inline spans through `markdown_inline`. HTML blocks inside Markdown still need the HTML pack, or the native fallback during migration.
- Adding or removing a baseline pack requires updating this record, the release packaging, and the registry index contract.
- C, C++, Ruby, and Ada are not baseline. They are on-demand packs after the registry exists. Every other language follows the same rule.

## Release gate

The later release check must verify:

- every baseline pack is in the release archive
- every baseline pack loads on every release target
- no non-baseline pack is bundled in the target state
- the registry index marks exactly these packs as baseline

Until the registry and release tooling exist, this record is the source of truth for the baseline.

## References

- `docs/design/wasm-grammars.md`
