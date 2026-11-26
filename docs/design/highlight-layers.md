# Highlight layers

Status: implemented.

## Problem

Diagnostics used to rewrite the whole style of a span. A search match, syntax token, plugin decoration, or bracket under a diagnostic lost its foreground and background; only the underline remained. Selection and syntax had the same issue: each painter owned a complete style instead of the attributes it needed.

## Model

Highlights use two axes:

- Color: foreground and background with explicit priority.
- Decoration: bold, dim, italic, crossed out, hidden, and underline. Each layer owns a subset.

`StylePatch` carries optional fields. Unset fields are left untouched, so a layer changes only what it owns. `UnderlinePatch` separates plain underlines from diagnostic underlines, which need both a kind and a color.

## Precedence

Foreground, low to high:

1. Base text.
2. Tagged diagnostic foreground (unnecessary or deprecated).
3. Syntax.
4. Plugin decorations.
5. Search match, current match, substitute.
6. Bracket match.

Selection has no foreground patch. Editor selection is background-only so syntax and search colors remain readable. `theme.selection.foreground` is still used by completion rows and overlays.

Background, low to high:

1. Base.
2. Syntax, when the theme sets it.
3. Plugin.
4. Search match, current match, substitute.
5. Selection.
6. Bracket match.

Decorations, last write wins per attribute:

- Syntax owns bold, italic, and underline.
- Plugin owns bold, dim, italic, crossed out, hidden, and underline.
- Search and selection own no decoration attributes.
- Bracket owns bold and clears dim, italic, crossed out, hidden, and underline.
- Diagnostics run last. Untagged diagnostics apply only an underline in the severity color. Unnecessary applies dim. Deprecated applies crossed out.

Tagged diagnostics do not draw an underline. Their foreground is a low-priority hint, so syntax and search colors win.

## Patch order

1. Tagged diagnostic foreground.
2. Syntax.
3. Special-key placeholders.
4. Plugin decorations and hidden-cursor reveal.
5. Search match, current match, substitute.
6. Selection background.
7. Bracket match.
8. Diagnostic decoration.

## Attribute ownership

- A layer only sets attributes it owns.
- Clearing an attribute is explicit with `Some(false)` or `UnderlinePatch::Clear`.
- `UnderlinePatch::Diagnostic(kind, color)` keeps the kind and color together, so a later diagnostic replaces both.
- Wide glyphs are patched from any covered cell. `Surface::patch_range` finds the glyph start and applies the patch once.

## Tests

- `surface/tests.rs`: requested fields only, unset attributes preserved, wide glyphs, edge clipping, underline replace and clear.
- `render/tests/diagnostic/overlay.rs`: each color layer survives a diagnostic underline.
- `render/tests/highlight/paint.rs`: syntax keeps the selection background; selection only changes the background.
- `render/tests/selection_wide.rs`: wide glyph selection alignment.
- `render/tests/virtual_text.rs`: plugin hidden style and cursor reveal.
