use std::ffi::{OsStr, OsString};
use std::fs::FileType;

use super::metadata::EntryMetadata;
use super::name::escape_name;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::app) enum EntryKind {
    File,
    Directory,
    Symlink,
    Fifo,
    Socket,
    BlockDevice,
    CharDevice,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) id: u64,
    pub(super) name: OsString,
    pub(super) kind: EntryKind,
    pub(super) symlink_target: Option<OsString>,
    pub(super) hidden: bool,
    pub(super) metadata: Option<EntryMetadata>,
}

impl Entry {
    pub(super) fn display_name(&self) -> String {
        let mut name = escape_name(self.name.as_os_str());
        if self.is_directory() {
            name.push('/');
        }
        name
    }

    pub(super) fn display_name_matches(&self, text: &str) -> bool {
        let text = if self.is_directory() {
            let Some(text) = text.strip_suffix('/') else {
                return false;
            };
            text
        } else {
            text
        };
        if needs_escape(&self.name) {
            return escape_name(self.name.as_os_str()) == text;
        }
        self.name == OsStr::new(text)
    }

    pub(super) fn is_directory(&self) -> bool {
        self.kind == EntryKind::Directory
    }

    pub(super) fn same_content(&self, other: &Self) -> bool {
        self.name == other.name
            && self.kind == other.kind
            && self.symlink_target == other.symlink_target
            && self.hidden == other.hidden
            && self.metadata == other.metadata
    }
}

fn needs_escape(name: &OsStr) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::str::from_utf8(name.as_bytes())
            .map(|text| text.chars().any(char_needs_escape))
            .unwrap_or(true)
    }
    #[cfg(not(unix))]
    {
        name.to_string_lossy().chars().any(char_needs_escape)
    }
}

fn char_needs_escape(character: char) -> bool {
    character == '\\' || character.is_control()
}

pub(super) fn is_hidden(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

pub(super) fn is_vcs_directory(name: &OsStr) -> bool {
    matches!(name.to_string_lossy().as_ref(), ".git" | ".hg" | ".svn")
}

pub(super) fn entry_kind(file_type: FileType) -> EntryKind {
    if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else {
        special_entry_kind(file_type)
    }
}

#[cfg(unix)]
fn special_entry_kind(file_type: FileType) -> EntryKind {
    use std::os::unix::fs::FileTypeExt;

    if file_type.is_fifo() {
        EntryKind::Fifo
    } else if file_type.is_socket() {
        EntryKind::Socket
    } else if file_type.is_block_device() {
        EntryKind::BlockDevice
    } else if file_type.is_char_device() {
        EntryKind::CharDevice
    } else {
        EntryKind::File
    }
}

#[cfg(not(unix))]
fn special_entry_kind(_file_type: FileType) -> EntryKind {
    EntryKind::File
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn display_names_match_plain_escaped_and_directory_names() {
        assert!(entry("alpha.txt", EntryKind::File).display_name_matches("alpha.txt"));
        assert!(entry("a\nb", EntryKind::File).display_name_matches("a\\nb"));
        assert!(entry("a\\b", EntryKind::File).display_name_matches("a\\\\b"));
        assert!(!entry("a\\b", EntryKind::File).display_name_matches("a\\b"));
        assert!(entry("src", EntryKind::Directory).display_name_matches("src/"));
        assert!(!entry("src", EntryKind::Directory).display_name_matches("src"));
        assert!(!entry("alpha.txt", EntryKind::File).display_name_matches("beta.txt"));
    }
}
