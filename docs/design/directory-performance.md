# Directory performance

Large directory listings are benchmarked with ignored tests in
`crates/kawi/src/app/directory/perf_tests.rs`. The harness covers filesystem
loading, watch-style reloads, sorting, rendering, diffing, planning, and executor
bookkeeping.

## Running the benchmarks

```sh
KAWI_PERF_ENTRIES=10000 cargo test --release -p kawi --no-default-features directory::perf_tests -- --ignored --nocapture --test-threads=1
KAWI_PERF_ENTRIES=50000 cargo test --release -p kawi --no-default-features directory::perf_tests -- --ignored --nocapture --test-threads=1
```

Run each size three times on an otherwise idle machine and compare medians. The
harness does not enforce thresholds in CI because timing is host dependent.

The load fixture creates regular files in the system temporary directory. The
sort, render, diff, plan, and apply fixtures use synthetic entries. Plan and apply
use the in-memory test filesystem, so they measure CPU bookkeeping rather than
storage latency.

## Baseline environment

Recorded on 2026-11-03.

- AMD Ryzen 3 7320U, 4 cores and 8 threads
- Linux 7.2.11-arch1-1
- rustc 1.99.1
- temporary directory on tmpfs
- release profile, single test thread

## Current baseline

| Operation | 10k entries | 50k entries |
| --- | ---: | ---: |
| Load with all metadata columns | 31.1 ms | 143.9 ms |
| Watch-style reload | 33.0 ms | 143.7 ms |
| Sort by name | 2.73 ms | 15.55 ms |
| Render listing text | 0.86 ms | 7.43 ms |
| Diff with every name changed | 19.3 ms | 117.7 ms |
| Diff with one changed name | 10.6 ms | 60.8 ms |
| Plan and apply 10k or 50k creates | 39.7 ms | 228.0 ms |
| Execute the generated plan | 56.1 ms | 337.1 ms |

## Recorded improvements

The initial audit measured 49.9 ms and 245.3 ms for loading 10k and 50k entries,
and 59.2 ms and 299.6 ms for fully changed diffs. Watch-style reloads measured
53.3 ms and 260.7 ms.

Metadata reads now run on up to eight workers for listings of at least 512
entries. Diffing uses hash collections, preallocated storage, and an allocation
free path for unchanged display names.

| Operation | 10k initial | 10k current | 50k initial | 50k current |
| --- | ---: | ---: | ---: | ---: |
| Load | 49.9 ms | 31.1 ms | 245.3 ms | 143.9 ms |
| Watch-style reload | 53.3 ms | 33.0 ms | 260.7 ms | 143.7 ms |
| Diff with every name changed | 59.2 ms | 19.3 ms | 299.6 ms | 117.7 ms |

Planning and executor bookkeeping were unchanged within run variance. They
remain the main CPU costs for very large bulk edits; real execution is dominated
by filesystem latency.
