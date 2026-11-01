use std::fs::Metadata;
use std::time::SystemTime;

use kawi_tui::text::display_width;

use super::entry::EntryKind;
use crate::options::{
    DIRECTORY_METADATA_GROUP, DIRECTORY_METADATA_LINKS, DIRECTORY_METADATA_MODIFIED,
    DIRECTORY_METADATA_OWNER, DIRECTORY_METADATA_PERMISSIONS, DIRECTORY_METADATA_SIZE,
};

mod format;
mod identity;
mod time;

use format::{
    format_group, format_links, format_owner, format_permissions, format_size, PERMISSION_WIDTH,
    SIZE_WIDTH,
};
use identity::IdentityResolver;
use time::{format_modified, MISSING_MODIFIED, MODIFIED_WIDTH};

const ENTRY_GAP: usize = 2;
const MIN_IDENTITY_WIDTH: usize = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) struct MetadataColumns {
    permissions: bool,
    links: bool,
    owner: bool,
    group: bool,
    size: bool,
    modified: bool,
}

impl MetadataColumns {
    pub(in crate::app) fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("none") {
            return Some(Self::default());
        }
        let mut columns = Self::default();
        for name in text.split(',').map(str::trim) {
            if name.is_empty() {
                return None;
            }
            match name {
                DIRECTORY_METADATA_PERMISSIONS => columns.permissions = true,
                DIRECTORY_METADATA_LINKS => columns.links = true,
                DIRECTORY_METADATA_OWNER => columns.owner = true,
                DIRECTORY_METADATA_GROUP => columns.group = true,
                DIRECTORY_METADATA_SIZE => columns.size = true,
                DIRECTORY_METADATA_MODIFIED => columns.modified = true,
                _ => return None,
            }
        }
        Some(columns)
    }

    pub(in crate::app) fn name(self) -> String {
        let mut names = Vec::new();
        if self.permissions {
            names.push(DIRECTORY_METADATA_PERMISSIONS);
        }
        if self.links {
            names.push(DIRECTORY_METADATA_LINKS);
        }
        if self.owner {
            names.push(DIRECTORY_METADATA_OWNER);
        }
        if self.group {
            names.push(DIRECTORY_METADATA_GROUP);
        }
        if self.size {
            names.push(DIRECTORY_METADATA_SIZE);
        }
        if self.modified {
            names.push(DIRECTORY_METADATA_MODIFIED);
        }
        if names.is_empty() {
            "none".to_string()
        } else {
            names.join(",")
        }
    }

    pub(in crate::app) fn all() -> Self {
        Self {
            permissions: true,
            links: true,
            owner: true,
            group: true,
            size: true,
            modified: true,
        }
    }

    pub(super) fn width(self, widths: MetadataWidths) -> usize {
        let columns = self.column_widths(widths);
        let count = columns.iter().flatten().count();
        if count == 0 {
            return 0;
        }
        let total = columns
            .iter()
            .flatten()
            .fold(0usize, |total, width| total.saturating_add(*width));
        total.saturating_add(count - 1).saturating_add(ENTRY_GAP)
    }

    pub(in crate::app) fn is_empty(self) -> bool {
        !self.permissions
            && !self.links
            && !self.owner
            && !self.group
            && !self.size
            && !self.modified
    }

    pub(super) fn any(self) -> bool {
        !self.is_empty()
    }

    fn column_widths(self, widths: MetadataWidths) -> [Option<usize>; 6] {
        [
            self.permissions.then_some(PERMISSION_WIDTH),
            self.links.then_some(widths.links),
            self.owner.then_some(widths.owner),
            self.group.then_some(widths.group),
            self.size.then_some(SIZE_WIDTH),
            self.modified.then_some(MODIFIED_WIDTH),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MetadataWidths {
    links: usize,
    owner: usize,
    group: usize,
}

impl Default for MetadataWidths {
    fn default() -> Self {
        Self {
            links: MIN_IDENTITY_WIDTH,
            owner: MIN_IDENTITY_WIDTH,
            group: MIN_IDENTITY_WIDTH,
        }
    }
}

impl MetadataWidths {
    pub(super) fn measure<'a>(entries: impl Iterator<Item = &'a EntryMetadata>) -> Self {
        entries.fold(Self::default(), |widths, metadata| widths.include(metadata))
    }

    fn include(self, metadata: &EntryMetadata) -> Self {
        Self {
            links: self.links.max(display_width(&metadata.links)),
            owner: self.owner.max(display_width(&metadata.owner)),
            group: self.group.max(display_width(&metadata.group)),
        }
    }
}

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
            size: format_size(metadata, kind),
            modified: metadata
                .modified()
                .ok()
                .map(|time| format_modified(time, self.now)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EntryMetadata {
    permissions: String,
    links: String,
    owner: String,
    group: String,
    size: String,
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
            push_padded(&mut text, &self.size, SIZE_WIDTH, Alignment::Right);
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
    fn parses_columns_and_rejects_unknown_names() {
        assert_eq!(
            MetadataColumns::parse("permissions,links,owner,group,size,modified"),
            Some(MetadataColumns::all())
        );
        assert!(MetadataColumns::parse("none").unwrap().is_empty());
        assert!(!MetadataColumns::parse("size").unwrap().modified);
        assert!(MetadataColumns::parse("mode").is_none());
        assert!(MetadataColumns::parse("size,").is_none());
    }

    #[test]
    fn names_columns_in_canonical_order() {
        let columns = MetadataColumns::parse("modified,group,owner,permissions").unwrap();
        assert_eq!(columns.name(), "permissions,owner,group,modified");
        assert_eq!(MetadataColumns::default().name(), "none");
    }

    #[test]
    fn columns_have_dynamic_display_widths() {
        let widths = MetadataWidths {
            links: 2,
            owner: 4,
            group: 6,
        };
        assert_eq!(MetadataColumns::all().width(widths), 48);
        assert_eq!(
            MetadataColumns::parse("permissions,size")
                .unwrap()
                .width(widths),
            20
        );
        assert_eq!(MetadataColumns::default().width(widths), 0);
    }

    #[test]
    fn metadata_text_follows_column_selection() {
        let metadata = metadata();
        let widths = MetadataWidths {
            links: 2,
            owner: 4,
            group: 5,
        };
        let columns = MetadataColumns::parse("links,owner,group,size,modified").unwrap();
        let text = metadata.text(columns, widths).unwrap();
        assert_eq!(text, " 2 user dev        13 Nov  1 20:26");
        assert_eq!(columns.width(widths), display_width(&text) + ENTRY_GAP);
        assert_eq!(metadata.text(MetadataColumns::default(), widths), None);
    }

    #[test]
    fn identity_widths_track_the_widest_value() {
        let mut widest = metadata();
        widest.links = "10".to_string();
        widest.owner = "alice".to_string();
        widest.group = "staff".to_string();
        let widths = MetadataWidths::measure([&metadata(), &widest].into_iter());
        assert_eq!(
            widths,
            MetadataWidths {
                links: 2,
                owner: 5,
                group: 5,
            }
        );
        assert_eq!(
            MetadataWidths::measure([].into_iter()),
            MetadataWidths::default()
        );
    }
}
