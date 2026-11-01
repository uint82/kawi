set_reproducible_rustflags() {
    rustc_hash=$(rustc -vV | sed -n 's/^commit-hash: //p')
    if [ -n "$rustc_hash" ]; then
        sysroot=$(rustc --print sysroot)
        RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$sysroot/lib/rustlib/src/rust=/rustc/$rustc_hash"
        export RUSTFLAGS
    fi
}
