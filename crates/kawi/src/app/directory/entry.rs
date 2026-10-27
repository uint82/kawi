use std::ffi::{OsStr, OsString};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum EntryKind {
    File,
    Directory,
    Symlink,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) id: u64,
    pub(super) name: OsString,
    pub(super) kind: EntryKind,
    pub(super) hidden: bool,
}

impl Entry {
    pub(super) fn display_name(&self) -> String {
        let mut name = self.name.to_string_lossy().into_owned();
        if self.kind == EntryKind::Directory {
            name.push('/');
        }
        name
    }

    pub(super) fn line_text(&self) -> String {
        self.display_name()
    }

    pub(super) fn is_directory(&self) -> bool {
        self.kind == EntryKind::Directory
    }
}

pub(super) fn is_hidden(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

pub(super) fn is_vcs_directory(name: &OsStr) -> bool {
    matches!(name.to_string_lossy().as_ref(), ".git" | ".hg" | ".svn")
}

pub(super) fn entry_kind(file_type: std::fs::FileType) -> EntryKind {
    if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::File
    }
}
