use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
fn identity(metadata: &fs::Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;

    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    modified: Option<std::time::SystemTime>,
    len: u64,
}

#[cfg(not(unix))]
fn identity(metadata: &fs::Metadata) -> FileIdentity {
    FileIdentity {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    }
}

#[cfg(unix)]
pub(super) fn file_identity(path: &Path) -> Option<FileIdentity> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::metadata(path).ok()?;
    (metadata.is_file() && metadata.nlink() == 1).then(|| identity(&metadata))
}

#[cfg(not(unix))]
pub(super) fn file_identity(path: &Path) -> Option<FileIdentity> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_file().then(|| identity(&metadata))
}

pub(super) fn directory_identity(path: &Path) -> Option<FileIdentity> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_dir().then(|| identity(&metadata))
}

pub(super) fn find_moved_entry(
    identity: FileIdentity,
    search: &BTreeSet<PathBuf>,
    old: &Path,
    lookup: fn(&Path) -> Option<FileIdentity>,
) -> Option<PathBuf> {
    let mut found = None;
    for directory in search {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path == old || lookup(&path) != Some(identity) {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(path);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn file_identities_follow_renames() {
        let parent = crate::test_support::temp_dir("identity_file_move");
        let old = parent.join("old.txt");
        let new = parent.join("new.txt");
        fs::write(&old, "content\n").unwrap();
        let identity = file_identity(&old).unwrap();
        fs::rename(&old, &new).unwrap();
        assert_eq!(file_identity(&new), Some(identity));
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn files_and_directories_have_distinct_identities() {
        let dir = crate::test_support::temp_dir("identity_kinds");
        let file = dir.join("alpha.txt");
        fs::write(&file, "content\n").unwrap();
        assert!(file_identity(&dir).is_none());
        assert!(directory_identity(&file).is_none());
        assert!(file_identity(&file).is_some());
        assert!(directory_identity(&dir).is_some());
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn hardlinked_files_have_no_unique_identity() {
        let dir = crate::test_support::temp_dir("identity_hardlink");
        let file = dir.join("alpha.txt");
        fs::write(&file, "content\n").unwrap();
        fs::hard_link(&file, dir.join("beta.txt")).unwrap();
        assert!(file_identity(&file).is_none());
        fs::remove_dir_all(dir).unwrap();
    }
}
