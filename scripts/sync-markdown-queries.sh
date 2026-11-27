#!/usr/bin/env sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
source="$root/crates/kawi-syntax/queries"
markdown="$root/crates/kawi-grammar/packs/markdown/0.5.4/queries"
inline="$root/crates/kawi-grammar/packs/markdown_inline/0.5.4/queries"

cp "$source/markdown.scm" "$markdown/highlights.scm"
cp "$source/markdown-injections.scm" "$markdown/injections.scm"
cp "$source/markdown-inline.scm" "$inline/highlights.scm"
cp "$source/markdown-inline-injections.scm" "$inline/injections.scm"
