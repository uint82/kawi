use std::path::{Path, PathBuf};

use ignore::{DirEntry, WalkBuilder};

const VCS_DIRECTORIES: &[&str] = &[".git", ".hg", ".svn"];

pub fn project_root(start: &Path) -> PathBuf {
    let start = absolute_start(start);
    let directory = if start.is_dir() {
        start
    } else {
        start.parent().map(Path::to_path_buf).unwrap_or(start)
    };
    find_repository_root(&directory)
}

pub fn relative_paths(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk_project(root, |path| {
        if let Some(relative) = relative_path(root, path) {
            files.push(relative);
        }
        true
    });
    files.sort();
    files
}

pub fn walk_project(root: &Path, mut visit: impl FnMut(&Path) -> bool) {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(not_vcs_entry);
    for entry in builder.build().flatten() {
        if !is_regular_file(&entry) {
            continue;
        }
        if !visit(entry.path()) {
            break;
        }
    }
}

fn absolute_start(start: &Path) -> PathBuf {
    let start = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(start))
            .unwrap_or_else(|_| start.to_path_buf())
    };
    start.canonicalize().unwrap_or(start)
}

fn find_repository_root(start: &Path) -> PathBuf {
    let mut current = start;
    loop {
        if current.join(".git").exists() {
            return current.to_path_buf();
        }
        let Some(parent) = current.parent() else {
            return start.to_path_buf();
        };
        current = parent;
    }
}

fn not_vcs_entry(entry: &DirEntry) -> bool {
    !VCS_DIRECTORIES
        .iter()
        .any(|name| entry.file_name() == *name)
}

fn is_regular_file(entry: &DirEntry) -> bool {
    entry.file_type().is_some_and(|kind| kind.is_file())
}

pub(crate) fn relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let text = relative.to_string_lossy().replace('\\', "/");
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_picker_{name}_{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub")).expect("project");
        fs::create_dir_all(root.join(".git")).expect("git");
        fs::write(root.join(".gitignore"), "ignored.txt\n").expect("ignore");
        fs::write(root.join("alpha.rs"), "fn main() {}\n").expect("alpha");
        fs::write(root.join("ignored.txt"), "ignored\n").expect("ignored");
        fs::write(root.join(".hidden.txt"), "hidden\n").expect("hidden");
        fs::write(root.join("sub/beta.rs"), "fn beta() {}\n").expect("beta");
        root
    }

    #[test]
    fn lists_tracked_hidden_and_nested_files() {
        let root = project("files");
        let files = relative_paths(&root);
        assert_eq!(
            files,
            vec![".gitignore", ".hidden.txt", "alpha.rs", "sub/beta.rs"]
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn finds_repository_root_from_nested_directory() {
        let root = project("root");
        let nested = root.join("sub");
        assert_eq!(project_root(&nested), root.canonicalize().unwrap());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn falls_back_to_the_starting_directory() {
        let root = project("fallback");
        fs::remove_dir_all(root.join(".git")).expect("remove git");
        assert_eq!(project_root(&root), root.canonicalize().unwrap());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
