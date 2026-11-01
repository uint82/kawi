#!/bin/sh
set -eu

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
target="$dir/target/wasm32-wasip2/release/config_fixture.wasm"

cargo build --manifest-path "$dir/Cargo.toml" --release --target wasm32-wasip2
cp "$target" "$dir/../config.wasm"
