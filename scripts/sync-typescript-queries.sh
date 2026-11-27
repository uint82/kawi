#!/usr/bin/env sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
pack="$root/crates/kawi-grammar/packs/typescript/0.23.2/queries/highlights.scm"
source="$root/crates/kawi-syntax/queries"
temporary="$pack.tmp"

trap 'rm -f "$temporary"' EXIT
{
    cat "$source/ecmascript.scm"
    printf '\n'
    cat "$source/ecmascript-tokens.scm"
    printf '\n'
    cat "$source/typescript.scm"
} > "$temporary"
mv "$temporary" "$pack"
trap - EXIT
