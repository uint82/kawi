use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub fn find_command(command: &str) -> Option<PathBuf> {
    if command.trim().is_empty() {
        return None;
    }
    let path = Path::new(command);
    if path.components().count() > 1 {
        return is_executable(path).then(|| path.to_path_buf());
    }
    find_in_paths(command, env::var_os("PATH").as_deref()?)
}

fn find_in_paths(command: &str, path: &OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .map(|directory| directory.join(command))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let id = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("kawi_lsp_binary_{name}_{stamp}_{id}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn executable(path: &Path) {
        fs::write(path, "#!/bin/sh\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("mode");
        }
    }

    #[test]
    fn finds_an_executable_on_path() {
        let dir = temp_dir("found");
        executable(&dir.join("server"));
        let path = env::join_paths([dir]).expect("path");
        assert_eq!(
            find_in_paths("server", &path),
            Some(env::split_paths(&path).next().unwrap().join("server"))
        );
    }

    #[test]
    fn ignores_missing_commands() {
        let dir = temp_dir("missing");
        let path = env::join_paths([dir]).expect("path");
        assert_eq!(find_in_paths("server", &path), None);
    }

    #[test]
    #[cfg(unix)]
    fn ignores_non_executable_files() {
        let dir = temp_dir("mode");
        let file = dir.join("server");
        fs::write(&file, "not executable").expect("write");
        let path = env::join_paths([dir]).expect("path");
        assert_eq!(find_in_paths("server", &path), None);
    }

    #[test]
    fn accepts_an_absolute_command() {
        let dir = temp_dir("absolute");
        let file = dir.join("server");
        executable(&file);
        assert_eq!(find_command(file.to_str().unwrap()), Some(file));
    }

    #[test]
    fn rejects_an_empty_command() {
        assert_eq!(find_command(""), None);
    }
}
