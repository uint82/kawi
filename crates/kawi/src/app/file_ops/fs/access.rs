use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_REMOVE_TREE_ENTRIES: usize = 4096;
const MAX_REMOVE_TREE_TIME: Duration = Duration::from_millis(50);

#[derive(Clone, Copy)]
enum Access {
    Read,
    Execute,
    Write,
    WriteExecute,
    ReadExecute,
    ReadWriteExecute,
}

pub(in crate::app) fn can_modify_directory(path: &Path) -> bool {
    access(path, Access::WriteExecute)
}

pub(in crate::app) fn can_write_directory(path: &Path) -> bool {
    access(path, Access::Write)
}

pub(in crate::app) fn can_read(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() {
        return path
            .parent()
            .is_some_and(|parent| access(parent, Access::Execute));
    }
    let mode = if metadata.is_dir() {
        Access::ReadExecute
    } else {
        Access::Read
    };
    access(path, mode)
}

pub(in crate::app) fn sticky_allows(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let Some(parent) = path.parent() else {
            return true;
        };
        let Ok(directory) = fs::symlink_metadata(parent) else {
            return false;
        };
        if directory.permissions().mode() & libc::S_ISVTX == 0 {
            return true;
        }
        let effective = unsafe { libc::geteuid() };
        if effective == 0 || directory.uid() == effective {
            return true;
        }
        fs::symlink_metadata(path).is_ok_and(|metadata| metadata.uid() == effective)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        true
    }
}

pub(in crate::app) fn can_remove_tree(path: &Path) -> bool {
    let mut walk = TreeWalk {
        remaining: MAX_REMOVE_TREE_ENTRIES,
        deadline: Instant::now() + MAX_REMOVE_TREE_TIME,
        pending: vec![path.to_path_buf()],
    };
    while let Some(path) = walk.pending.pop() {
        match walk.step(&path) {
            TreeStep::Continue => {}
            TreeStep::Allowed => return true,
            TreeStep::Denied => return false,
        }
    }
    true
}

struct TreeWalk {
    remaining: usize,
    deadline: Instant,
    pending: Vec<PathBuf>,
}

enum TreeStep {
    Continue,
    Allowed,
    Denied,
}

impl TreeWalk {
    fn step(&mut self, path: &Path) -> TreeStep {
        if Instant::now() >= self.deadline || self.remaining == 0 {
            return TreeStep::Allowed;
        }
        if !sticky_allows(path) {
            return TreeStep::Denied;
        }
        self.remaining -= 1;
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return TreeStep::Denied;
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return TreeStep::Continue;
        }
        if !access(path, Access::ReadWriteExecute) {
            return TreeStep::Denied;
        }
        self.push_children(path)
    }

    fn push_children(&mut self, path: &Path) -> TreeStep {
        let Ok(entries) = fs::read_dir(path) else {
            return TreeStep::Denied;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                return TreeStep::Denied;
            };
            self.pending.push(entry.path());
            if self.pending.len() >= MAX_REMOVE_TREE_ENTRIES {
                return TreeStep::Allowed;
            }
        }
        TreeStep::Continue
    }
}

#[cfg(unix)]
fn access(path: &Path, mode: Access) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    let mode = match mode {
        Access::Read => libc::R_OK,
        Access::Execute => libc::X_OK,
        Access::Write => libc::W_OK,
        Access::WriteExecute => libc::W_OK | libc::X_OK,
        Access::ReadExecute => libc::R_OK | libc::X_OK,
        Access::ReadWriteExecute => libc::R_OK | libc::W_OK | libc::X_OK,
    };
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        unsafe { libc::faccessat(libc::AT_FDCWD, path.as_ptr(), mode, libc::AT_EACCESS) == 0 }
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        unsafe { libc::access(path.as_ptr(), mode) == 0 }
    }
}

#[cfg(not(unix))]
fn access(path: &Path, mode: Access) -> bool {
    match mode {
        Access::Read | Access::Execute | Access::ReadExecute => true,
        Access::Write | Access::WriteExecute | Access::ReadWriteExecute => {
            fs::metadata(path).is_ok_and(|metadata| !metadata.permissions().readonly())
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn directory_write_access_requires_execute() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = temp_dir("access_write");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o200)).unwrap();
        assert!(!can_modify_directory(&dir));
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(can_modify_directory(&dir));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unreadable_files_are_not_readable() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = temp_dir("access_read");
        let file = dir.join("file.txt");
        fs::write(&file, "content\n").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
        assert!(!can_read(&file));
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(can_read(&file));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sticky_directories_allow_owned_entries() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = temp_dir("access_sticky");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o1777)).unwrap();
        let file = dir.join("owned.txt");
        fs::write(&file, "content\n").unwrap();
        assert!(sticky_allows(&file));
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn tree_removal_does_not_follow_symlinks() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = temp_dir("access_symlink");
        let tree = dir.join("tree");
        let outside = dir.join("outside");
        fs::create_dir_all(&tree).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o500)).unwrap();
        std::os::unix::fs::symlink(&outside, tree.join("link")).unwrap();
        assert!(can_remove_tree(&tree));
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o700)).unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn tree_removal_requires_writable_directories() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = temp_dir("access_remove");
        let tree = dir.join("tree");
        let child = tree.join("child");
        fs::create_dir_all(&child).unwrap();
        fs::write(child.join("file.txt"), "content\n").unwrap();
        fs::set_permissions(&child, fs::Permissions::from_mode(0o500)).unwrap();
        assert!(!can_remove_tree(&tree));
        fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(can_remove_tree(&tree));
        let _ = fs::remove_dir_all(dir);
    }
}
