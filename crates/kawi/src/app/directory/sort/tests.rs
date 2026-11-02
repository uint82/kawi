use std::cmp::Ordering;
use std::ffi::OsString;
use std::time::{Duration, SystemTime};

use super::super::entry::{Entry, EntryKind};
use super::super::metadata::EntryMetadata;
use super::*;

fn entry(name: &str, kind: EntryKind) -> Entry {
    Entry {
        id: 1,
        name: OsString::from(name),
        kind,
        symlink_target: None,
        hidden: false,
        metadata: None,
    }
}

fn sized(name: &str, size: u64) -> Entry {
    let mut entry = entry(name, EntryKind::File);
    entry.metadata = Some(metadata(size, None));
    entry
}

fn metadata(size: u64, modified_time: Option<SystemTime>) -> EntryMetadata {
    EntryMetadata {
        permissions: String::new(),
        links: String::new(),
        owner: String::new(),
        group: String::new(),
        size: String::new(),
        size_bytes: size,
        modified: None,
        modified_time,
    }
}

fn options(key: SortKey) -> SortOptions {
    SortOptions {
        key,
        reverse: false,
        directories_first: false,
    }
}

#[test]
fn natural_order_handles_numbers() {
    assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
    assert_eq!(natural_cmp("File10", "file2"), Ordering::Greater);
    assert_eq!(natural_cmp("file02", "file2"), Ordering::Greater);
}

#[test]
fn size_sort_orders_smallest_first() {
    let small = sized("small", 2);
    let large = sized("large", 10);
    assert_eq!(
        compare_entries(&small, &large, options(SortKey::Size)),
        Ordering::Less
    );
    assert_eq!(
        compare_entries(&large, &small, options(SortKey::Size)),
        Ordering::Greater
    );
}

#[test]
fn missing_metadata_sorts_last() {
    let known = sized("known", 2);
    let unknown = entry("unknown", EntryKind::File);
    assert_eq!(
        compare_entries(&known, &unknown, options(SortKey::Size)),
        Ordering::Less
    );
    assert_eq!(
        compare_entries(&unknown, &known, options(SortKey::Size)),
        Ordering::Greater
    );
}

#[test]
fn modified_sort_orders_oldest_first() {
    let now = SystemTime::UNIX_EPOCH;
    let mut old = entry("old", EntryKind::File);
    old.metadata = Some(metadata(0, Some(now)));
    let mut new = entry("new", EntryKind::File);
    new.metadata = Some(metadata(0, Some(now + Duration::from_secs(10))));
    assert_eq!(
        compare_entries(&old, &new, options(SortKey::Modified)),
        Ordering::Less
    );
}

#[test]
fn extension_sort_uses_natural_order() {
    let txt = entry("file10.txt", EntryKind::File);
    let zip = entry("file2.zip", EntryKind::File);
    assert_eq!(
        compare_entries(&txt, &zip, options(SortKey::Extension)),
        Ordering::Less
    );
}

#[test]
fn extension_sort_places_entries_without_extension_first() {
    let plain = entry("plain", EntryKind::File);
    let txt = entry("file.txt", EntryKind::File);
    assert_eq!(
        compare_entries(&plain, &txt, options(SortKey::Extension)),
        Ordering::Less
    );
}

#[test]
fn kind_sort_uses_file_type_rank() {
    let file = entry("file", EntryKind::File);
    let directory = entry("directory", EntryKind::Directory);
    let socket = entry("socket", EntryKind::Socket);
    assert_eq!(
        compare_entries(&directory, &file, options(SortKey::Kind)),
        Ordering::Less
    );
    assert_eq!(
        compare_entries(&file, &socket, options(SortKey::Kind)),
        Ordering::Less
    );
}

#[test]
fn reverse_keeps_directories_first() {
    let directory = entry("zdir", EntryKind::Directory);
    let large = sized("large", 10);
    let small = sized("small", 1);
    let sort = SortOptions {
        key: SortKey::Size,
        reverse: true,
        directories_first: true,
    };
    assert_eq!(compare_entries(&directory, &large, sort), Ordering::Less);
    assert_eq!(compare_entries(&large, &small, sort), Ordering::Less);
}

#[test]
fn name_is_the_stable_tie_breaker() {
    let left = sized("alpha", 1);
    let right = sized("beta", 1);
    assert_eq!(
        compare_entries(&left, &right, options(SortKey::Size)),
        Ordering::Less
    );
}
