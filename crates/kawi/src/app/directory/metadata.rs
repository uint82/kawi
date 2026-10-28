use std::fs::Metadata;

use super::entry::EntryKind;
use crate::options::{
    DIRECTORY_METADATA_MODIFIED, DIRECTORY_METADATA_PERMISSIONS, DIRECTORY_METADATA_SIZE,
};

mod format;

use format::{
    format_modified, format_permissions, format_size, MISSING_MODIFIED, MODIFIED_WIDTH,
    PERMISSION_WIDTH, SIZE_WIDTH, TEXT_WIDTH,
};

const ENTRY_GAP: usize = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) struct MetadataColumns {
    permissions: bool,
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
            size: true,
            modified: true,
        }
    }

    pub(in crate::app) fn width(self) -> usize {
        let mut width = 0usize;
        let mut count = 0usize;
        if self.permissions {
            width += PERMISSION_WIDTH;
            count += 1;
        }
        if self.size {
            width += SIZE_WIDTH;
            count += 1;
        }
        if self.modified {
            width += MODIFIED_WIDTH;
            count += 1;
        }
        if count == 0 {
            return 0;
        }
        width
            .saturating_add(count.saturating_sub(1))
            .saturating_add(ENTRY_GAP)
    }

    pub(in crate::app) fn is_empty(self) -> bool {
        !self.permissions && !self.size && !self.modified
    }

    pub(super) fn any(self) -> bool {
        !self.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EntryMetadata {
    permissions: String,
    size: String,
    modified: Option<String>,
}

impl EntryMetadata {
    pub(super) fn read(metadata: &Metadata, kind: EntryKind) -> Self {
        Self {
            permissions: format_permissions(metadata, kind),
            size: format_size(metadata, kind),
            modified: metadata.modified().ok().map(format_modified),
        }
    }

    pub(super) fn text(&self, columns: MetadataColumns) -> Option<String> {
        let mut text = String::with_capacity(TEXT_WIDTH);
        if columns.permissions {
            push_field(&mut text, &self.permissions);
        }
        if columns.size {
            push_field(&mut text, &self.size);
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

fn push_field(text: &mut String, field: &str) {
    if !text.is_empty() {
        text.push(' ');
    }
    text.push_str(field);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_columns_and_rejects_unknown_names() {
        assert_eq!(
            MetadataColumns::parse("permissions,size,modified"),
            Some(MetadataColumns {
                permissions: true,
                size: true,
                modified: true,
            })
        );
        assert!(MetadataColumns::parse("none").unwrap().is_empty());
        assert!(!MetadataColumns::parse("size").unwrap().modified);
        assert!(MetadataColumns::parse("owner").is_none());
        assert!(MetadataColumns::parse("size,").is_none());
    }

    #[test]
    fn names_columns_in_canonical_order() {
        let columns = MetadataColumns::parse("modified,permissions").unwrap();
        assert_eq!(columns.name(), "permissions,modified");
        assert_eq!(MetadataColumns::default().name(), "none");
    }

    #[test]
    fn columns_have_fixed_display_widths() {
        assert_eq!(MetadataColumns::all().width(), 37);
        assert_eq!(
            MetadataColumns::parse("permissions,size").unwrap().width(),
            20
        );
        assert_eq!(MetadataColumns::default().width(), 0);
    }

    #[test]
    fn metadata_text_follows_column_selection() {
        let metadata = EntryMetadata {
            permissions: "-rw-r--r--".to_string(),
            size: "     13".to_string(),
            modified: Some("2026-10-28 14:32".to_string()),
        };
        let columns = MetadataColumns::parse("size,modified").unwrap();
        assert_eq!(
            metadata.text(columns).as_deref(),
            Some("     13 2026-10-28 14:32")
        );
        assert_eq!(metadata.text(MetadataColumns::default()), None);
    }
}
