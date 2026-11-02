use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::app::identity::{directory_identity, FileIdentity};

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

pub(super) fn root_identity_changed(path: &Path, previous: Option<FileIdentity>) -> bool {
    previous != directory_identity(path)
}

pub(super) fn root_missing(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => !metadata.is_dir(),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::app::identity::find_moved_entry;

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
        let identity = directory_identity(&dir);
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
    fn moved_roots_are_found_in_search_paths() {
        let parent = crate::test_support::temp_dir("directory_watch_moved");
        let old = parent.join("old");
        let new = parent.join("new");
        fs::create_dir_all(&old).unwrap();
        let identity = directory_identity(&old).unwrap();
        fs::rename(&old, &new).unwrap();
        let roots = BTreeSet::from([old.clone()]);
        let search = watched_paths(&roots);
        assert_eq!(
            find_moved_entry(identity, &search, &old, directory_identity),
            Some(new)
        );
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn replaced_roots_are_not_treated_as_moves() {
        let parent = crate::test_support::temp_dir("directory_watch_replaced");
        let old = parent.join("old");
        fs::create_dir_all(&old).unwrap();
        let identity = directory_identity(&old).unwrap();
        fs::remove_dir_all(&old).unwrap();
        fs::create_dir_all(&old).unwrap();
        let roots = BTreeSet::from([old.clone()]);
        let search = watched_paths(&roots);
        assert_eq!(
            find_moved_entry(identity, &search, &old, directory_identity),
            None
        );
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn replacement_changes_the_root_identity() {
        let dir = crate::test_support::temp_dir("directory_watch_identity_replace");
        let replacement = crate::test_support::temp_dir("directory_watch_identity_new");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(&replacement).unwrap();
        let identity = directory_identity(&dir);
        fs::remove_dir_all(&dir).unwrap();
        fs::rename(&replacement, &dir).unwrap();
        assert!(root_identity_changed(&dir, identity));
        fs::remove_dir_all(dir).unwrap();
    }
}
