#!/bin/sh
set -eu

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
manifest="$dir/network/Cargo.toml"
target="$dir/network/target/wasm32-wasip2/release/network_fixture.wasm"

cargo build --manifest-path "$manifest" --release --target wasm32-wasip2
cp "$target" "$dir/network.wasm"
