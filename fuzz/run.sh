#!/bin/sh
set -eu

mode=${1:-smoke}
case "$mode" in
    smoke)
        seconds=${FUZZ_SECONDS:-30}
        rss=${FUZZ_RSS_MB:-2048}
        malloc=${FUZZ_MALLOC_MB:-256}
        ;;
    nightly)
        seconds=${FUZZ_SECONDS:-300}
        rss=${FUZZ_RSS_MB:-4096}
        malloc=${FUZZ_MALLOC_MB:-512}
        ;;
    *)
        echo "usage: $0 smoke|nightly" >&2
        exit 2
        ;;
esac

max_len() {
    case "$1" in
        network_policy) echo 4096 ;;
        stream_bytes) echo 65536 ;;
        process_framing) echo 1048576 ;;
        authorization) echo 1024 ;;
        host_imports) echo 262144 ;;
        process_run) echo 4096 ;;
    esac
}

directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$directory"

for target in network_policy stream_bytes process_framing authorization host_imports process_run; do
    length=$(max_len "$target")
    cargo fuzz run "$target" -- \
        -dict=dictionaries/protocol.dict \
        -max_total_time="$seconds" \
        -max_len="$length" \
        -rss_limit_mb="$rss" \
        -malloc_limit_mb="$malloc" \
        -timeout=10 \
        -print_final_stats=1
done
