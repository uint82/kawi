use std::path::Path;

pub(in crate::app) fn can_modify_directory(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        let mode = libc::W_OK | libc::X_OK;
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
    {
        std::fs::metadata(path)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.permissions().readonly())
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
}
