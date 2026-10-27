use std::path::Path;

pub(super) fn is_relevant(path: &Path, git_dir: &Path) -> bool {
    if path == git_dir {
        return true;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        "HEAD"
            | "index"
            | "FETCH_HEAD"
            | "packed-refs"
            | "ORIG_HEAD"
            | "MERGE_HEAD"
            | "CHERRY_PICK_HEAD"
            | "REVERT_HEAD"
            | "BISECT_LOG"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_relevance_is_limited_to_git_state() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant(&git_dir.join("HEAD"), git_dir));
        assert!(is_relevant(&git_dir.join("index"), git_dir));
        assert!(is_relevant(git_dir, git_dir));
        assert!(!is_relevant(&git_dir.join("objects"), git_dir));
    }
}
