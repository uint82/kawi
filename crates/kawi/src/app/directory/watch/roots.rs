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
    Some(RootIdentity {
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
    Some(RootIdentity {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

pub(super) fn root_identity_changed(path: &Path, previous: Option<RootIdentity>) -> bool {
    previous != root_identity(path)
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
