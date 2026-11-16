#!/bin/sh
set -eu

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
target="$dir/target/wasm32-wasip2/release/tasks_fixture.wasm"

. "$dir/../rustflags.sh"
set_reproducible_rustflags

cargo build --manifest-path "$dir/Cargo.toml" --release --target wasm32-wasip2
cp "$target" "$dir/../tasks-plugin.wasm"
