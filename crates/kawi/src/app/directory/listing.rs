use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use std::time::SystemTime;

use super::entry::{entry_kind, is_hidden, is_vcs_directory, Entry, EntryKind};
use super::metadata::{EntryMetadata, MetadataColumns, MetadataReader, MetadataWidths};
use super::sort::{compare_entries, SortOptions};

const PARALLEL_METADATA_MIN_ENTRIES: usize = 512;
const MAX_METADATA_WORKERS: usize = 8;

pub(super) struct Listing {
    pub(super) entries: Vec<Entry>,
    pub(super) next_id: u64,
    pub(super) metadata_widths: MetadataWidths,
}

impl Listing {
    pub(super) fn empty() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
            metadata_widths: MetadataWidths::default(),
        }
    }
}

pub(super) fn listing_text(listing: &Listing) -> String {
    let capacity: usize = listing
        .entries
        .iter()
        .map(|entry| entry.name.len().saturating_add(2))
        .sum();
    let mut text = String::with_capacity(capacity);
    for (index, entry) in listing.entries.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str(&entry.display_name());
    }
    text
}

pub(super) fn listing_unchanged(original: &[Entry], current: &Listing) -> bool {
    original.len() == current.entries.len()
        && original
            .iter()
            .zip(&current.entries)
            .all(|(old, new)| old.name == new.name && old.kind == new.kind)
}

pub(super) fn read_listing(
    root: &Path,
    show_hidden: bool,
    first_id: u64,
    sort: SortOptions,
    columns: MetadataColumns,
) -> io::Result<Listing> {
    let settings = ListingSettings::new(show_hidden, columns, sort);
    let (mut entries, next_id) = read_entries(root, first_id, &settings)?;
    if settings.needs_metadata {
        read_metadata(root, &mut entries, columns);
    }
    entries.sort_by(|left, right| compare_entries(left, right, sort));
    let metadata_widths =
        MetadataWidths::measure(entries.iter().filter_map(|entry| entry.metadata.as_ref()));
    Ok(Listing {
        entries,
        next_id,
        metadata_widths,
    })
}

struct ListingSettings {
    show_hidden: bool,
    needs_metadata: bool,
}

impl ListingSettings {
    fn new(show_hidden: bool, columns: MetadataColumns, sort: SortOptions) -> Self {
        Self {
            show_hidden,
            needs_metadata: columns.any() || sort.key.needs_metadata(),
        }
    }
}

fn read_entries(
    root: &Path,
    first_id: u64,
    settings: &ListingSettings,
) -> io::Result<(Vec<Entry>, u64)> {
    let mut entries = Vec::new();
    let mut next_id = first_id;
    for item in fs::read_dir(root)? {
        if let Some(entry) = read_entry(item?, next_id, settings.show_hidden)? {
            entries.push(entry);
            next_id = next_id.saturating_add(1);
        }
    }
    Ok((entries, next_id))
}

fn read_entry(item: fs::DirEntry, id: u64, show_hidden: bool) -> io::Result<Option<Entry>> {
    let name = item.file_name();
    let hidden = is_hidden(&name);
    let kind = entry_kind(item.file_type()?);
    if kind == EntryKind::Directory && is_vcs_directory(&name) {
        return Ok(None);
    }
    if hidden && !show_hidden {
        return Ok(None);
    }
    let symlink_target = if kind == EntryKind::Symlink {
        fs::read_link(item.path())
            .ok()
            .map(|path| path.into_os_string())
    } else {
        None
    };
    Ok(Some(Entry {
        id,
        name,
        kind,
        symlink_target,
        hidden,
        metadata: None,
    }))
}

fn read_metadata(root: &Path, entries: &mut [Entry], columns: MetadataColumns) {
    let workers = metadata_worker_count(entries.len());
    if workers <= 1 {
        read_metadata_range(root, entries, columns, SystemTime::now());
        return;
    }
    let now = SystemTime::now();
    let chunk_size = entries.len().div_ceil(workers);
    thread::scope(|scope| {
        for chunk in entries.chunks_mut(chunk_size) {
            scope.spawn(move || read_metadata_range(root, chunk, columns, now));
        }
    });
}

fn metadata_worker_count(entries: usize) -> usize {
    if entries < PARALLEL_METADATA_MIN_ENTRIES {
        return 1;
    }
    thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(MAX_METADATA_WORKERS)
        .min(entries)
}

fn read_metadata_range(
    root: &Path,
    entries: &mut [Entry],
    columns: MetadataColumns,
    now: SystemTime,
) {
    let mut reader = MetadataReader::new(now);
    for entry in entries {
        entry.metadata = read_entry_metadata(root, entry, columns, &mut reader);
    }
}

fn read_entry_metadata(
    root: &Path,
    entry: &Entry,
    columns: MetadataColumns,
    reader: &mut MetadataReader,
) -> Option<EntryMetadata> {
    let metadata = fs::symlink_metadata(root.join(&entry.name)).ok()?;
    Some(reader.read(&metadata, entry.kind, columns))
}

#[cfg(test)]
mod tests;
