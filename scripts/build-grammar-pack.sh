#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 <grammar-source-dir> <pack-dir>" >&2
    exit 2
fi

source_dir=$1
pack_dir=$2
tree_sitter=${TREE_SITTER_CLI:-tree-sitter}
wasm_tools=${WASM_TOOLS_CLI:-wasm-tools}
wasm=$pack_dir/grammar.wasm
temporary=$pack_dir/.grammar.wasm.tmp

if [ ! -f "$pack_dir/manifest.toml" ]; then
    echo "missing manifest: $pack_dir/manifest.toml" >&2
    exit 1
fi

trap 'rm -f "$temporary"' EXIT
"$tree_sitter" build --wasm --output "$temporary" "$source_dir"
"$wasm_tools" validate "$temporary"
mv "$temporary" "$wasm"
trap - EXIT

if command -v sha256sum >/dev/null 2>&1; then
    hash=$(sha256sum "$wasm" | cut -d' ' -f1)
else
    hash=$(shasum -a 256 "$wasm" | cut -d' ' -f1)
fi
size=$(wc -c < "$wasm" | tr -d ' ')

printf 'wasm: %s\nsha256: %s\nsize: %s bytes\n' "$wasm" "$hash" "$size"
echo "update manifest.toml and PROVENANCE.md with these values" >&2
