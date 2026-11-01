use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::Path;

use super::entry::{entry_kind, is_hidden, is_vcs_directory, Entry, EntryKind};
use super::metadata::{MetadataColumns, MetadataReader, MetadataWidths};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) enum SortMode {
    #[default]
    DirectoriesFirst,
    Name,
}

impl SortMode {
    pub(in crate::app) fn parse(value: &str) -> Option<Self> {
        match value {
            "directories" => Some(Self::DirectoriesFirst),
            "name" => Some(Self::Name),
            _ => None,
        }
    }

    pub(in crate::app) fn name(self) -> &'static str {
        match self {
            Self::DirectoriesFirst => "directories",
            Self::Name => "name",
        }
    }
}

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
        text.push_str(&entry.line_text());
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
    let mut reader = MetadataReader::new();
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

fn compare_entries(left: &Entry, right: &Entry, sort: SortMode) -> Ordering {
    let directories = if sort == SortMode::DirectoriesFirst {
        right.is_directory().cmp(&left.is_directory())
    } else {
        Ordering::Equal
    };
    directories
        .then_with(|| natural_cmp(&left.name.to_string_lossy(), &right.name.to_string_lossy()))
        .then_with(|| left.name.cmp(&right.name))
}

pub(super) fn natural_cmp(left_text: &str, right_text: &str) -> Ordering {
    let left: Vec<char> = left_text.chars().collect();
    let right: Vec<char> = right_text.chars().collect();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        let left_char = left[left_index];
        let right_char = right[right_index];
        if left_char.is_ascii_digit() && right_char.is_ascii_digit() {
            let left_end = digit_end(&left, left_index);
            let right_end = digit_end(&right, right_index);
            let left_digits = trim_zeroes(&left[left_index..left_end]);
            let right_digits = trim_zeroes(&right[right_index..right_end]);
            let ordering = left_digits
                .len()
                .cmp(&right_digits.len())
                .then_with(|| left_digits.cmp(right_digits))
                .then_with(|| (left_end - left_index).cmp(&(right_end - right_index)));
            if ordering != Ordering::Equal {
                return ordering;
            }
            left_index = left_end;
            right_index = right_end;
            continue;
        }
        let ordering = lower(left_char).cmp(&lower(right_char));
        if ordering != Ordering::Equal {
            return ordering;
        }
        left_index += 1;
        right_index += 1;
    }
    (left.len() - left_index)
        .cmp(&(right.len() - right_index))
        .then_with(|| left_text.cmp(right_text))
}

fn digit_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    end
}

fn trim_zeroes(digits: &[char]) -> &[char] {
    let start = digits
        .iter()
        .position(|digit| *digit != '0')
        .unwrap_or(digits.len());
    &digits[start..]
}

fn lower(character: char) -> char {
    character.to_lowercase().next().unwrap_or(character)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::test_support::temp_dir;

    use super::*;

    #[test]
    fn natural_order_handles_numbers() {
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("File10", "file2"), Ordering::Greater);
        assert_eq!(natural_cmp("file02", "file2"), Ordering::Greater);
    }

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
        assert!(text.contains("5B"));
        assert_eq!(text.len(), 35);
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

    #[cfg(unix)]
    #[test]
    fn identity_columns_show_link_count_owner_and_group() {
        let dir = temp_dir("directory_identity");
        fs::write(dir.join("alpha.txt"), "hello").unwrap();
        let columns = MetadataColumns::parse("links,owner,group").unwrap();
        let listing = read_listing(&dir, false, 1, SortMode::DirectoriesFirst, columns).unwrap();
        let entry = listing
            .entries
            .iter()
            .find(|entry| entry.name == "alpha.txt")
            .unwrap();
        let text = entry
            .metadata
            .as_ref()
            .unwrap()
            .text(columns, listing.metadata_widths)
            .unwrap();
        let fields: Vec<&str> = text.split_whitespace().collect();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "1");
        assert!(!fields[1].is_empty());
        assert!(!fields[2].is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
}
