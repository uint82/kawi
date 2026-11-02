use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn watched_paths(roots: &BTreeSet<PathBuf>) -> BTreeSet<PathBuf> {
    let mut watched = roots.clone();
    watched.extend(
        roots
            .iter()
            .filter_map(|root| root.parent())
            .map(Path::to_path_buf),
    );
    watched
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RootIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
pub(super) fn root_identity(path: &Path) -> Option<RootIdentity> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::metadata(path).ok()?;
    metadata.is_dir().then(|| RootIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RootIdentity {
    modified: Option<std::time::SystemTime>,
    len: u64,
}

#[cfg(not(unix))]
pub(super) fn root_identity(path: &Path) -> Option<RootIdentity> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_dir().then(|| RootIdentity {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

pub(super) fn root_identity_changed(path: &Path, previous: Option<RootIdentity>) -> bool {
    previous != root_identity(path)
}

pub(super) fn root_missing(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => !metadata.is_dir(),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

pub(super) fn move_search_paths(roots: &BTreeSet<PathBuf>, old: &Path) -> BTreeSet<PathBuf> {
    let mut search = BTreeSet::new();
    if let Some(parent) = old.parent() {
        search.insert(parent.to_path_buf());
    }
    for root in roots {
        search.insert(root.clone());
        if let Some(parent) = root.parent() {
            search.insert(parent.to_path_buf());
        }
    }
    search
}

pub(super) fn find_moved_root(
    identity: RootIdentity,
    search: &BTreeSet<PathBuf>,
    old: &Path,
) -> Option<PathBuf> {
    let mut found = None;
    for directory in search {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path == old || root_identity(&path) != Some(identity) {
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
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn watched_paths_include_each_parent() {
        let roots = BTreeSet::from([
            PathBuf::from("/project/src"),
            PathBuf::from("/project/tests"),
        ]);
        let watched = watched_paths(&roots);
        assert!(watched.contains(&PathBuf::from("/project")));
        assert!(watched.contains(&PathBuf::from("/project/src")));
        assert!(watched.contains(&PathBuf::from("/project/tests")));
    }

    #[test]
    fn content_changes_keep_the_root_identity() {
        let dir = crate::test_support::temp_dir("directory_watch_identity_content");
        fs::create_dir_all(&dir).unwrap();
        let identity = root_identity(&dir);
        fs::write(dir.join("alpha.txt"), "content\n").unwrap();
        assert!(!root_identity_changed(&dir, identity));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn deleted_roots_are_missing() {
        let dir = crate::test_support::temp_dir("directory_watch_missing");
        fs::remove_dir_all(&dir).unwrap();
        assert!(root_missing(&dir));
    }

    #[test]
    fn files_have_no_root_identity() {
        let dir = crate::test_support::temp_dir("directory_watch_file_identity");
        let file = dir.join("alpha.txt");
        fs::write(&file, "content\n").unwrap();
        assert_eq!(root_identity(&file), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn moved_roots_are_found_in_search_paths() {
        let parent = crate::test_support::temp_dir("directory_watch_moved");
        let old = parent.join("old");
        let new = parent.join("new");
        fs::create_dir_all(&old).unwrap();
        let identity = root_identity(&old).unwrap();
        fs::rename(&old, &new).unwrap();
        let roots = BTreeSet::from([old.clone()]);
        let search = move_search_paths(&roots, &old);
        assert_eq!(find_moved_root(identity, &search, &old), Some(new));
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn replaced_roots_are_not_treated_as_moves() {
        let parent = crate::test_support::temp_dir("directory_watch_replaced");
        let old = parent.join("old");
        fs::create_dir_all(&old).unwrap();
        let identity = root_identity(&old).unwrap();
        fs::remove_dir_all(&old).unwrap();
        fs::create_dir_all(&old).unwrap();
        let roots = BTreeSet::from([old.clone()]);
        let search = move_search_paths(&roots, &old);
        assert_eq!(find_moved_root(identity, &search, &old), None);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn replacement_changes_the_root_identity() {
        let dir = crate::test_support::temp_dir("directory_watch_identity_replace");
        let replacement = crate::test_support::temp_dir("directory_watch_identity_new");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(&replacement).unwrap();
        let identity = root_identity(&dir);
        fs::remove_dir_all(&dir).unwrap();
        fs::rename(&replacement, &dir).unwrap();
        assert!(root_identity_changed(&dir, identity));
        fs::remove_dir_all(dir).unwrap();
    }
}
