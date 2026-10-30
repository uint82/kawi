#!/bin/sh
set -eu

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
manifest="$dir/network/Cargo.toml"
target="$dir/network/target/wasm32-wasip2/release/network_fixture.wasm"

rustc_hash=$(rustc -vV | sed -n 's/^commit-hash: //p')
if [ -n "$rustc_hash" ]; then
    sysroot=$(rustc --print sysroot)
    export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$sysroot/lib/rustlib/src/rust=/rustc/$rustc_hash"
fi

cargo build --manifest-path "$manifest" --release --target wasm32-wasip2
cp "$target" "$dir/network.wasm"
