use std::path::{Component, Path, PathBuf};

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn absolute(path: &str, cwd: &Path) -> PathBuf {
    let joined = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        cwd.join(path)
    };
    normalize(&joined)
}

pub fn shorten(path: &Path, cwd: &Path) -> String {
    match path.strip_prefix(cwd) {
        Ok(rest) if !rest.as_os_str().is_empty() => rest.to_string_lossy().into_owned(),
        _ => path.to_string_lossy().into_owned(),
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn cwd() -> PathBuf {
        PathBuf::from("/tmp/kawi/paths")
    }

    #[test]
    fn absolute_joins_relative_paths() {
        assert_eq!(absolute("a.txt", &cwd()), cwd().join("a.txt"));
        assert_eq!(absolute("sub/a.txt", &cwd()), cwd().join("sub/a.txt"));
    }

    #[test]
    fn absolute_normalizes_dot_segments() {
        assert_eq!(absolute("./a.txt", &cwd()), cwd().join("a.txt"));
        assert_eq!(
            absolute("../b.txt", &cwd()),
            PathBuf::from("/tmp/kawi/b.txt")
        );
        assert_eq!(absolute("sub/../a.txt", &cwd()), cwd().join("a.txt"));
    }

    #[test]
    fn absolute_keeps_absolute_input() {
        assert_eq!(absolute("/etc/hosts", &cwd()), PathBuf::from("/etc/hosts"));
        assert_eq!(
            absolute("/etc/./hosts", &cwd()),
            PathBuf::from("/etc/hosts")
        );
    }

    #[test]
    fn shorten_strips_the_cwd_prefix() {
        let path = cwd().join("sub/a.txt");
        assert_eq!(shorten(&path, &cwd()), "sub/a.txt");
    }

    #[test]
    fn shorten_keeps_paths_outside_the_cwd() {
        assert_eq!(shorten(Path::new("/etc/hosts"), &cwd()), "/etc/hosts");
        assert_eq!(
            shorten(Path::new("/tmp/other.txt"), &cwd()),
            "/tmp/other.txt"
        );
    }
}
