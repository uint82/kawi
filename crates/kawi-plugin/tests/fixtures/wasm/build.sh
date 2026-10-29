#!/bin/sh
set -eu

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

for wat in "$dir"/src/*.wat; do
    name=$(basename "$wat" .wat)
    wasm-tools parse "$wat" -o "$dir/$name.wasm"
done
