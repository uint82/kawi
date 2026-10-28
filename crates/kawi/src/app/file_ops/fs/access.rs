use std::fs;
use std::path::Path;

#[derive(Clone, Copy)]
enum Access {
    Read,
    Execute,
    WriteExecute,
    ReadExecute,
    ReadWriteExecute,
}

pub(in crate::app) fn can_modify_directory(path: &Path) -> bool {
    access(path, Access::WriteExecute)
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

pub(in crate::app) fn can_remove_tree(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return true;
    }
    if !access(path, Access::ReadWriteExecute) {
        return false;
    }
    let Ok(mut entries) = fs::read_dir(path) else {
        return false;
    };
    entries.all(|entry| entry.is_ok_and(|entry| can_remove_tree(&entry.path())))
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
        Access::WriteExecute | Access::ReadWriteExecute => {
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
