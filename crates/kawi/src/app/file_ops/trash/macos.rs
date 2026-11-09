use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::topdir::{ensure_root, is_directory, same_device, user_id};
use super::{is_writable_location, set_private, unique_name, TrashError};

const TRASH_DIR: &str = ".Trash";
const SHARED_TRASH_DIR: &str = ".Trashes";
const VOLUME_TRASH_PREFIX: &str = ".Trash-";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Location {
    #[cfg(target_os = "macos")]
    Home,
    Volume,
}

pub(super) struct MacOsTrash {
    root: PathBuf,
    location: Location,
}

impl MacOsTrash {
    #[cfg(target_os = "macos")]
    pub(super) fn for_target(target: &Path) -> Option<Self> {
        Self::home()
            .filter(|trash| trash.can_trash(target))
            .or_else(|| Self::volume(target))
    }

    #[cfg(target_os = "macos")]
    fn home() -> Option<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())?;
        Some(Self {
            root: home_root(&home),
            location: Location::Home,
        })
    }

    #[cfg(target_os = "macos")]
    fn volume(target: &Path) -> Option<Self> {
        let topdir = super::topdir::mount_point(target)?;
        Some(Self {
            root: volume_root(&topdir)?,
            location: Location::Volume,
        })
    }

    #[cfg(test)]
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            location: Location::Volume,
        }
    }

    pub(super) fn can_trash(&self, target: &Path) -> bool {
        same_device(target, &self.root) && is_writable_location(&self.root)
    }

    pub(super) fn trash_as(&self, target: &Path) -> Result<PathBuf, TrashError> {
        self.ensure()?;
        if !same_device(target, &self.root) {
            return Err(TrashError::Unavailable(
                "trash is on another filesystem".to_string(),
            ));
        }
        let root = &self.root;
        let name = unique_name(target, |name| !root.join(name).exists())?;
        let destination = root.join(name);
        fs::rename(target, &destination)
            .map(|()| destination)
            .map_err(TrashError::Io)
    }

    fn ensure(&self) -> io::Result<()> {
        let created = ensure_root(&self.root)?;
        if created || matches!(self.location, Location::Volume) {
            set_private(&self.root)?;
        }
        Ok(())
    }
}

pub(super) fn home_root(home: &Path) -> PathBuf {
    home.join(TRASH_DIR)
}

pub(super) fn volume_root(topdir: &Path) -> Option<PathBuf> {
    let user = user_id()?;
    let shared = topdir.join(SHARED_TRASH_DIR);
    if is_directory(&shared) && is_writable_location(&shared) {
        Some(shared.join(user))
    } else {
        Some(topdir.join(format!("{VOLUME_TRASH_PREFIX}{user}")))
    }
}
