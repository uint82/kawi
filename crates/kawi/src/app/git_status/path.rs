use std::path::{Path, PathBuf};

use kawi_git::Status;

pub(super) fn absolute_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| canonical_parent(path))
}

fn canonical_parent(path: &Path) -> PathBuf {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = std::fs::canonicalize(parent)
        .or_else(|_| std::path::absolute(parent))
        .unwrap_or_else(|_| parent.to_path_buf());
    match path.file_name() {
        Some(name) => parent.join(name),
        None => parent,
    }
}

pub(super) fn status_matches_path(path: Option<&Path>, status: &Status) -> bool {
    path.is_some_and(|path| path.starts_with(&status.root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolves_an_existing_file() {
        let dir = crate::test_support::temp_dir("git_status_path");
        let file = dir.join("a.txt");
        fs::write(&file, "").expect("write");
        assert_eq!(
            absolute_path(&file),
            dir.canonicalize().expect("canonical").join("a.txt")
        );
    }

    #[test]
    fn resolves_a_missing_file_under_an_existing_parent() {
        let dir = crate::test_support::temp_dir("git_status_missing");
        let file = dir.join("missing.txt");
        assert_eq!(
            absolute_path(&file),
            dir.canonicalize().expect("canonical").join("missing.txt")
        );
    }
}
