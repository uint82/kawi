use std::fs::Metadata;
use std::time::SystemTime;

use kawi_tui::text::display_width;

use super::entry::EntryKind;

mod columns;
mod format;
mod identity;
mod time;

pub(in crate::app) use columns::MetadataColumns;
pub(super) use columns::MetadataWidths;
use format::{format_group, format_links, format_owner, format_permissions, format_size};
use identity::IdentityResolver;
use time::{format_modified, MISSING_MODIFIED};

pub(super) struct MetadataReader {
    identities: IdentityResolver,
    now: SystemTime,
}

impl MetadataReader {
    pub(super) fn new(now: SystemTime) -> Self {
        Self {
            identities: IdentityResolver::new(),
            now,
        }
    }

    pub(super) fn read(&mut self, metadata: &Metadata, kind: EntryKind) -> EntryMetadata {
        EntryMetadata {
            permissions: format_permissions(metadata, kind),
            links: format_links(metadata),
            owner: format_owner(metadata, &mut self.identities),
            group: format_group(metadata, &mut self.identities),
            size: format_size(metadata),
            modified: metadata
                .modified()
                .ok()
                .map(|time| format_modified(time, self.now)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EntryMetadata {
    pub(super) permissions: String,
    pub(super) links: String,
    pub(super) owner: String,
    pub(super) group: String,
    pub(super) size: String,
    modified: Option<String>,
}

impl EntryMetadata {
    pub(super) fn text(&self, columns: MetadataColumns, widths: MetadataWidths) -> Option<String> {
        let mut text = String::with_capacity(columns.width(widths));
        if columns.permissions {
            push_field(&mut text, &self.permissions);
        }
        if columns.links {
            push_padded(&mut text, &self.links, widths.links, Alignment::Right);
        }
        if columns.owner {
            push_padded(&mut text, &self.owner, widths.owner, Alignment::Left);
        }
        if columns.group {
            push_padded(&mut text, &self.group, widths.group, Alignment::Left);
        }
        if columns.size {
            push_padded(&mut text, &self.size, widths.size, Alignment::Right);
        }
        if columns.modified {
            push_field(
                &mut text,
                self.modified.as_deref().unwrap_or(MISSING_MODIFIED),
            );
        }
        (!text.is_empty()).then_some(text)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Alignment {
    Left,
    Right,
}

fn push_field(text: &mut String, field: &str) {
    if !text.is_empty() {
        text.push(' ');
    }
    text.push_str(field);
}

fn push_padded(text: &mut String, field: &str, width: usize, alignment: Alignment) {
    if !text.is_empty() {
        text.push(' ');
    }
    let padding = width.saturating_sub(display_width(field));
    if alignment == Alignment::Right {
        text.extend(std::iter::repeat_n(' ', padding));
    }
    text.push_str(field);
    if alignment == Alignment::Left {
        text.extend(std::iter::repeat_n(' ', padding));
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::test_support::temp_dir;

    use super::super::listing::read_listing;
    use super::super::sort::SortMode;
    use super::*;

    fn metadata() -> EntryMetadata {
        EntryMetadata {
            permissions: "-rw-r--r--".to_string(),
            links: "2".to_string(),
            owner: "user".to_string(),
            group: "dev".to_string(),
            size: "13".to_string(),
            modified: Some("Nov  1 20:26".to_string()),
        }
    }

    #[test]
    fn metadata_text_follows_column_selection() {
        let metadata = metadata();
        let widths = MetadataWidths {
            links: 2,
            owner: 4,
            group: 5,
            size: 2,
        };
        let columns = MetadataColumns::parse("links,owner,group,size,modified").unwrap();
        let text = metadata.text(columns, widths).unwrap();
        assert_eq!(text, " 2 user dev   13 Nov  1 20:26");
        assert_eq!(columns.width(widths), display_width(&text) + 2);
        assert_eq!(metadata.text(MetadataColumns::default(), widths), None);
    }

    #[test]
    fn dynamic_widths_track_the_widest_value() {
        let mut widest = metadata();
        widest.links = "10".to_string();
        widest.owner = "alice".to_string();
        widest.group = "staff".to_string();
        widest.size = "12345".to_string();
        let widths = MetadataWidths::measure([&metadata(), &widest].into_iter());
        assert_eq!(
            widths,
            MetadataWidths {
                links: 2,
                owner: 5,
                group: 5,
                size: 5,
            }
        );
        assert_eq!(
            MetadataWidths::measure(std::iter::empty()),
            MetadataWidths::default()
        );
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
