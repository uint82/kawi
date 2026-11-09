use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(any(not(target_os = "macos"), test))]
mod freedesktop;
#[cfg(any(target_os = "macos", test))]
mod macos;
#[cfg(test)]
mod tests;
mod topdir;

const MAX_NAME_ATTEMPTS: u32 = 10_000;
const DEFAULT_NAME: &str = "item";

#[derive(Debug)]
pub(in crate::app) enum TrashError {
    Unavailable(String),
    Io(io::Error),
}

impl std::fmt::Display for TrashError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) => formatter.write_str(message),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl From<io::Error> for TrashError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub(in crate::app) struct TrashDir {
    backend: Backend,
}

enum Backend {
    #[cfg(any(not(target_os = "macos"), test))]
    Freedesktop(freedesktop::FreedesktopTrash),
    #[cfg(any(target_os = "macos", test))]
    MacOs(macos::MacOsTrash),
}

impl TrashDir {
    pub(in crate::app) fn for_target(target: &Path) -> Option<Self> {
        #[cfg(target_os = "macos")]
        {
            macos::MacOsTrash::for_target(target).map(|trash| Self {
                backend: Backend::MacOs(trash),
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            freedesktop::FreedesktopTrash::for_target(target).map(|trash| Self {
                backend: Backend::Freedesktop(trash),
            })
        }
    }

    #[cfg(test)]
    fn freedesktop(root: PathBuf) -> Self {
        Self {
            backend: Backend::Freedesktop(freedesktop::FreedesktopTrash::new(root)),
        }
    }

    #[cfg(test)]
    fn macos(root: PathBuf) -> Self {
        Self {
            backend: Backend::MacOs(macos::MacOsTrash::new(root)),
        }
    }

    pub(in crate::app) fn can_trash(&self, target: &Path) -> bool {
        match &self.backend {
            #[cfg(any(not(target_os = "macos"), test))]
            Backend::Freedesktop(trash) => trash.can_trash(target),
            #[cfg(any(target_os = "macos", test))]
            Backend::MacOs(trash) => trash.can_trash(target),
        }
    }

    pub(in crate::app) fn trash_as(
        &self,
        target: &Path,
        original: &Path,
    ) -> Result<PathBuf, TrashError> {
        match &self.backend {
            #[cfg(any(not(target_os = "macos"), test))]
            Backend::Freedesktop(trash) => trash.trash_as(target, original),
            #[cfg(any(target_os = "macos", test))]
            Backend::MacOs(trash) => trash.trash_as(target),
        }
    }
}

fn is_writable_location(path: &Path) -> bool {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return false;
                }
                return super::fs::can_modify_directory(ancestor);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return false,
        }
    }
    false
}

fn unique_name(target: &Path, is_available: impl Fn(&str) -> bool) -> io::Result<String> {
    let base = base_name(target);
    for index in 0..MAX_NAME_ATTEMPTS {
        let name = candidate_name(&base, index);
        if is_available(&name) {
            return Ok(name);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "trash contains too many items with the same name",
    ))
}

fn base_name(target: &Path) -> String {
    target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| DEFAULT_NAME.to_string())
}

fn candidate_name(base: &str, index: u32) -> String {
    if index == 0 {
        base.to_string()
    } else {
        format!("{base}.{index}")
    }
}

#[cfg(unix)]
fn set_private(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private(_path: &Path) -> io::Result<()> {
    Ok(())
}
