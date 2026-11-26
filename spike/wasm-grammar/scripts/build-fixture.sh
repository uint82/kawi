#!/usr/bin/env sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <tree-sitter-typescript checkout>" >&2
    exit 2
fi

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tree_sitter=${TREE_SITTER_CLI:-tree-sitter}

"$tree_sitter" build --wasm --output "$root/fixtures/typescript.wasm" "$1/typescript"
sha256sum "$root/fixtures/typescript.wasm"
