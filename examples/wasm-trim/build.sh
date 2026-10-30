#!/bin/sh
set -eu

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$dir/../.." && pwd)

cargo run --quiet --manifest-path "$root/Cargo.toml" -p kawi -- plugin build "$dir"
