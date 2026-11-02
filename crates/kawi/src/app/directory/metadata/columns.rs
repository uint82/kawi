use kawi_tui::text::display_width;

use super::format::PERMISSION_WIDTH;
use super::time::MODIFIED_WIDTH;
use super::EntryMetadata;
use crate::options::{
    DIRECTORY_METADATA_GROUP, DIRECTORY_METADATA_LINKS, DIRECTORY_METADATA_MODIFIED,
    DIRECTORY_METADATA_OWNER, DIRECTORY_METADATA_PERMISSIONS, DIRECTORY_METADATA_SIZE,
};

const ENTRY_GAP: usize = 2;
const MIN_DYNAMIC_WIDTH: usize = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) struct MetadataColumns {
    pub(super) permissions: bool,
    pub(super) links: bool,
    pub(super) owner: bool,
    pub(super) group: bool,
    pub(super) size: bool,
    pub(super) modified: bool,
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

    pub(in crate::app::directory) fn width(self, widths: MetadataWidths) -> usize {
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

    pub(in crate::app::directory) fn any(self) -> bool {
        !self.is_empty()
    }

    fn column_widths(self, widths: MetadataWidths) -> [Option<usize>; 6] {
        [
            self.permissions.then_some(PERMISSION_WIDTH),
            self.links.then_some(widths.links),
            self.owner.then_some(widths.owner),
            self.group.then_some(widths.group),
            self.size.then_some(widths.size),
            self.modified.then_some(MODIFIED_WIDTH),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app::directory) struct MetadataWidths {
    pub(super) links: usize,
    pub(super) owner: usize,
    pub(super) group: usize,
    pub(super) size: usize,
}

impl Default for MetadataWidths {
    fn default() -> Self {
        Self {
            links: MIN_DYNAMIC_WIDTH,
            owner: MIN_DYNAMIC_WIDTH,
            group: MIN_DYNAMIC_WIDTH,
            size: MIN_DYNAMIC_WIDTH,
        }
    }
}

impl MetadataWidths {
    pub(in crate::app::directory) fn measure<'a>(
        entries: impl Iterator<Item = &'a EntryMetadata>,
    ) -> Self {
        entries.fold(Self::default(), |widths, metadata| widths.include(metadata))
    }

    fn include(self, metadata: &EntryMetadata) -> Self {
        Self {
            links: self.links.max(display_width(&metadata.links)),
            owner: self.owner.max(display_width(&metadata.owner)),
            group: self.group.max(display_width(&metadata.group)),
            size: self.size.max(display_width(&metadata.size)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            size: 3,
        };
        assert_eq!(MetadataColumns::all().width(widths), 44);
        assert_eq!(
            MetadataColumns::parse("permissions,size")
                .unwrap()
                .width(widths),
            16
        );
        assert_eq!(MetadataColumns::default().width(widths), 0);
    }
}
