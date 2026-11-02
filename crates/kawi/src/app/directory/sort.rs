use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::Path;

use super::entry::{Entry, EntryKind};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) enum SortKey {
    #[default]
    Name,
    Size,
    Modified,
    Extension,
    Kind,
}

impl SortKey {
    pub(in crate::app) fn parse(value: &str) -> Option<Self> {
        match value {
            "name" | "directories" => Some(Self::Name),
            "size" => Some(Self::Size),
            "modified" => Some(Self::Modified),
            "extension" => Some(Self::Extension),
            "kind" => Some(Self::Kind),
            _ => None,
        }
    }

    pub(in crate::app) fn name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Size => "size",
            Self::Modified => "modified",
            Self::Extension => "extension",
            Self::Kind => "kind",
        }
    }

    pub(super) fn needs_metadata(self) -> bool {
        matches!(self, Self::Size | Self::Modified)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) struct SortOptions {
    pub(in crate::app) key: SortKey,
    pub(in crate::app) reverse: bool,
    pub(in crate::app) directories_first: bool,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            key: SortKey::Name,
            reverse: false,
            directories_first: true,
        }
    }
}

pub(super) fn compare_entries(left: &Entry, right: &Entry, sort: SortOptions) -> Ordering {
    let grouped = grouped_order(left, right, sort.directories_first);
    grouped.then_with(|| {
        let ordering = compare_keys(left, right, sort.key)
            .then_with(|| natural_cmp(&left.name.to_string_lossy(), &right.name.to_string_lossy()))
            .then_with(|| left.name.cmp(&right.name));
        if sort.reverse {
            ordering.reverse()
        } else {
            ordering
        }
    })
}

fn grouped_order(left: &Entry, right: &Entry, directories_first: bool) -> Ordering {
    if directories_first {
        right.is_directory().cmp(&left.is_directory())
    } else {
        Ordering::Equal
    }
}

fn compare_keys(left: &Entry, right: &Entry, key: SortKey) -> Ordering {
    match key {
        SortKey::Name => Ordering::Equal,
        SortKey::Size => compare_optional(entry_size(left), entry_size(right)),
        SortKey::Modified => compare_optional(entry_modified(left), entry_modified(right)),
        SortKey::Extension => compare_extensions(left, right),
        SortKey::Kind => kind_rank(left.kind).cmp(&kind_rank(right.kind)),
    }
}

fn entry_size(entry: &Entry) -> Option<u64> {
    Some(entry.metadata.as_ref()?.size_bytes)
}

fn entry_modified(entry: &Entry) -> Option<std::time::SystemTime> {
    entry.metadata.as_ref()?.modified_time
}

fn compare_extensions(left: &Entry, right: &Entry) -> Ordering {
    match (extension_text(left), extension_text(right)) {
        (Some(left), Some(right)) => natural_cmp(&left, &right),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn extension_text(entry: &Entry) -> Option<Cow<'_, str>> {
    Path::new(&entry.name)
        .extension()
        .map(|extension| extension.to_string_lossy())
}

fn kind_rank(kind: EntryKind) -> u8 {
    match kind {
        EntryKind::Directory => 0,
        EntryKind::File => 1,
        EntryKind::Symlink => 2,
        EntryKind::Fifo => 3,
        EntryKind::Socket => 4,
        EntryKind::BlockDevice => 5,
        EntryKind::CharDevice => 6,
    }
}

fn compare_optional<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

pub(super) fn natural_cmp(left: &str, right: &str) -> Ordering {
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left_bytes.len() && right_index < right_bytes.len() {
        let left_char = left[left_index..].chars().next().expect("character");
        let right_char = right[right_index..].chars().next().expect("character");
        if left_char.is_ascii_digit() && right_char.is_ascii_digit() {
            let left_end = digit_end(left_bytes, left_index);
            let right_end = digit_end(right_bytes, right_index);
            let ordering = compare_digit_runs(
                &left_bytes[left_index..left_end],
                &right_bytes[right_index..right_end],
            );
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
        left_index += left_char.len_utf8();
        right_index += right_char.len_utf8();
    }
    (left_bytes.len() - left_index)
        .cmp(&(right_bytes.len() - right_index))
        .then_with(|| left.cmp(right))
}

fn digit_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    end
}

fn compare_digit_runs(left: &[u8], right: &[u8]) -> Ordering {
    let left_padded = left.len();
    let right_padded = right.len();
    let left = trim_zeroes(left);
    let right = trim_zeroes(right);
    left.len()
        .cmp(&right.len())
        .then_with(|| left.cmp(right))
        .then_with(|| left_padded.cmp(&right_padded))
}

fn trim_zeroes(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| *byte != b'0')
        .unwrap_or(bytes.len());
    &bytes[start..]
}

fn lower(character: char) -> char {
    character.to_lowercase().next().unwrap_or(character)
}

#[cfg(test)]
mod tests;
