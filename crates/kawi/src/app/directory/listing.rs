use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use super::entry::{entry_kind, is_hidden, is_vcs_directory, Entry, EntryKind};
use super::metadata::{MetadataColumns, MetadataReader, MetadataWidths};
use super::sort::{compare_entries, SortOptions};

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
    let mut reader = MetadataReader::new(SystemTime::now());
    let mut entries = Vec::new();
    let mut next_id = first_id;
    for item in fs::read_dir(root)? {
        if let Some(entry) = read_entry(item?, next_id, &settings, &mut reader)? {
            entries.push(entry);
            next_id = next_id.saturating_add(1);
        }
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
    columns: MetadataColumns,
    needs_metadata: bool,
}

impl ListingSettings {
    fn new(show_hidden: bool, columns: MetadataColumns, sort: SortOptions) -> Self {
        Self {
            show_hidden,
            columns,
            needs_metadata: columns.any() || sort.key.needs_metadata(),
        }
    }
}

fn read_entry(
    item: fs::DirEntry,
    id: u64,
    settings: &ListingSettings,
    reader: &mut MetadataReader,
) -> io::Result<Option<Entry>> {
    let name = item.file_name();
    let hidden = is_hidden(&name);
    let kind = entry_kind(item.file_type()?);
    if kind == EntryKind::Directory && is_vcs_directory(&name) {
        return Ok(None);
    }
    if hidden && !settings.show_hidden {
        return Ok(None);
    }
    let metadata = if settings.needs_metadata {
        fs::symlink_metadata(item.path()).ok()
    } else {
        None
    };
    let symlink_target = if kind == EntryKind::Symlink {
        fs::read_link(item.path())
            .ok()
            .map(|path| path.into_os_string())
    } else {
        None
    };
    let metadata = metadata
        .as_ref()
        .map(|metadata| reader.read(metadata, kind, settings.columns));
    Ok(Some(Entry {
        id,
        name,
        kind,
        symlink_target,
        hidden,
        metadata,
    }))
}

#[cfg(test)]
mod tests;
