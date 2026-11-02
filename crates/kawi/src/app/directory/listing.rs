use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use super::entry::{entry_kind, is_hidden, is_vcs_directory, Entry, EntryKind};
use super::metadata::{MetadataColumns, MetadataReader, MetadataWidths};
use super::sort::{compare_entries, SortMode};

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
    sort: SortMode,
    columns: MetadataColumns,
) -> io::Result<Listing> {
    let mut entries = Vec::new();
    let mut next_id = first_id;
    let mut reader = MetadataReader::new(SystemTime::now());
    for item in fs::read_dir(root)? {
        let item = item?;
        let name = item.file_name();
        let hidden = is_hidden(&name);
        let metadata = if columns.any() {
            item.metadata().ok()
        } else {
            None
        };
        let file_type = match metadata.as_ref() {
            Some(metadata) => metadata.file_type(),
            None => item.file_type()?,
        };
        let kind = entry_kind(file_type);
        if kind == EntryKind::Directory && is_vcs_directory(&name) {
            continue;
        }
        if hidden && !show_hidden {
            continue;
        }
        entries.push(Entry {
            id: next_id,
            name,
            kind,
            hidden,
            metadata: metadata
                .as_ref()
                .map(|metadata| reader.read(metadata, kind)),
        });
        next_id = next_id.saturating_add(1);
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

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::test_support::temp_dir;

    use super::*;

    #[test]
    fn name_sort_ignores_entry_kind() {
        let dir = temp_dir("directory_name_sort");
        fs::create_dir_all(dir.join("zdir")).unwrap();
        fs::write(dir.join("alpha.txt"), "").unwrap();
        let listing =
            read_listing(&dir, false, 1, SortMode::Name, MetadataColumns::default()).unwrap();
        let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
        assert_eq!(names, ["alpha.txt", "zdir/"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn directories_sort_before_files_and_hidden_entries_are_filtered() {
        let dir = temp_dir("directory_listing");
        fs::create_dir_all(dir.join("zeta")).unwrap();
        fs::write(dir.join("alpha.txt"), "").unwrap();
        fs::write(dir.join(".hidden"), "").unwrap();
        fs::write(dir.join("beta10.txt"), "").unwrap();
        fs::write(dir.join("beta2.txt"), "").unwrap();
        let listing = read_listing(
            &dir,
            false,
            1,
            SortMode::DirectoriesFirst,
            MetadataColumns::default(),
        )
        .unwrap();
        let names: Vec<String> = listing.entries.iter().map(Entry::display_name).collect();
        assert_eq!(names, ["zeta/", "alpha.txt", "beta2.txt", "beta10.txt"]);
        let listing = read_listing(
            &dir,
            true,
            1,
            SortMode::DirectoriesFirst,
            MetadataColumns::default(),
        )
        .unwrap();
        assert!(listing.entries.iter().any(|entry| entry.name == ".hidden"));
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn metadata_is_read_only_when_columns_are_enabled() {
        let dir = temp_dir("directory_metadata");
        fs::write(dir.join("alpha.txt"), "hello").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        let listing = read_listing(
            &dir,
            false,
            1,
            SortMode::DirectoriesFirst,
            MetadataColumns::default(),
        )
        .unwrap();
        assert!(listing.entries.iter().all(|entry| entry.metadata.is_none()));
        let columns = MetadataColumns::parse("permissions,size,modified").unwrap();
        let listing = read_listing(&dir, false, 1, SortMode::DirectoriesFirst, columns).unwrap();
        let file = listing
            .entries
            .iter()
            .find(|entry| entry.name == "alpha.txt")
            .unwrap();
        let text = file
            .metadata
            .as_ref()
            .unwrap()
            .text(columns, listing.metadata_widths)
            .unwrap();
        assert!(text.starts_with('-'));
        let fields: Vec<&str> = text.split_whitespace().collect();
        assert_eq!(fields.get(1), Some(&"5"));
        let directory = listing
            .entries
            .iter()
            .find(|entry| entry.name == "sub")
            .unwrap();
        let text = directory
            .metadata
            .as_ref()
            .unwrap()
            .text(columns, listing.metadata_widths)
            .unwrap();
        assert!(text.starts_with('d'));
        fs::remove_dir_all(dir).unwrap();
    }
}
