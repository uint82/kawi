use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::Instant;

use super::diff::{diff, Change, DirectoryLine};
use super::entry::{Entry, EntryKind};
use super::executor::execute;
use super::listing::{listing_text, read_listing, Listing};
use super::metadata::{MetadataColumns, MetadataWidths};
use super::planner::{plan, Operation};
use super::sort::{compare_entries, SortOptions};
use super::test_support::FakeFs;
use crate::test_support::temp_dir;

const DEFAULT_ENTRIES: usize = 10_000;
const ENTRY_COUNT_ENV: &str = "KAWI_PERF_ENTRIES";
const ROOT: &str = "/project";

fn entry_count() -> usize {
    std::env::var(ENTRY_COUNT_ENV)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|count: &usize| *count > 0)
        .unwrap_or(DEFAULT_ENTRIES)
}

fn timed<T>(label: &str, entries: usize, run: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let value = run();
    eprintln!("{label} entries={entries} elapsed={:?}", start.elapsed());
    value
}

fn synthetic_entry(id: u64) -> Entry {
    Entry {
        id,
        name: OsString::from(format!("file-{id:06}.txt")),
        kind: EntryKind::File,
        symlink_target: None,
        hidden: false,
        metadata: None,
    }
}

fn synthetic_entries(count: usize) -> Vec<Entry> {
    (0..count)
        .map(|index| synthetic_entry(index as u64))
        .collect()
}

fn create_files(dir: &Path, count: usize) {
    for index in 0..count {
        fs::write(dir.join(format!("file-{index:06}.txt")), "").unwrap();
    }
}

#[test]
#[ignore = "manual performance benchmark"]
fn benchmark_large_directory_load() {
    let count = entry_count();
    let dir = temp_dir("directory_perf_load");
    create_files(&dir, count);
    let load = || {
        read_listing(
            &dir,
            false,
            1,
            SortOptions::default(),
            MetadataColumns::all(),
        )
        .unwrap()
    };
    let listing = timed("load", count, load);
    assert_eq!(listing.entries.len(), count);
    let refreshed = timed("watch-refresh", count, load);
    assert_eq!(refreshed.entries.len(), count);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "manual performance benchmark"]
fn benchmark_large_directory_sort() {
    let count = entry_count();
    let mut entries = synthetic_entries(count);
    entries.reverse();
    timed("sort", count, || {
        entries.sort_by(|left, right| compare_entries(left, right, SortOptions::default()));
    });
    assert_eq!(entries.first().unwrap().id, 0);
}

#[test]
#[ignore = "manual performance benchmark"]
fn benchmark_large_directory_render() {
    let count = entry_count();
    let listing = Listing {
        entries: synthetic_entries(count),
        next_id: count as u64 + 1,
        metadata_widths: MetadataWidths::default(),
    };
    let text = timed("render", count, || listing_text(&listing));
    assert!(text.len() >= count);
}

#[test]
#[ignore = "manual performance benchmark"]
fn benchmark_large_directory_diff() {
    let count = entry_count();
    let root = Path::new(ROOT);
    let entries = synthetic_entries(count);
    let lines: Vec<DirectoryLine> = entries
        .iter()
        .map(|entry| DirectoryLine {
            id: Some(entry.id),
            text: format!("renamed-{}", entry.display_name()),
        })
        .collect();
    let changes = timed("diff-all-changed", count, || {
        diff(root, &entries, &lines).unwrap()
    });
    assert_eq!(changes.len(), count);
    let mut sparse: Vec<DirectoryLine> = entries
        .iter()
        .map(|entry| DirectoryLine {
            id: Some(entry.id),
            text: entry.display_name(),
        })
        .collect();
    sparse[0].text = "renamed-file-000000.txt".to_string();
    let changes = timed("diff-sparse", count, || {
        diff(root, &entries, &sparse).unwrap()
    });
    assert_eq!(changes.len(), 1);
}

#[test]
#[ignore = "manual performance benchmark"]
fn benchmark_large_directory_plan_and_apply() {
    let count = entry_count();
    let root = Path::new(ROOT);
    let changes: Vec<Change> = (0..count)
        .map(|index| Change::Create {
            path: root.join(format!("new-{index:06}.txt")),
            kind: EntryKind::File,
        })
        .collect();
    let fs = FakeFs::new(&[(ROOT, true)]);
    let plan = timed("plan", count, || plan(changes, &fs).unwrap());
    let mut fs = FakeFs::new(&[(ROOT, true)]);
    let outcome = timed("apply", count, || execute(&plan, &mut fs));
    assert_eq!(
        outcome.remaining,
        (0..plan.operations.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        plan.operations
            .iter()
            .filter(|operation| matches!(operation, Operation::CreateFile(_)))
            .count(),
        count
    );
    assert_eq!(outcome.completed, plan.operations.len());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.rollback_errors, Vec::<String>::new());
}
