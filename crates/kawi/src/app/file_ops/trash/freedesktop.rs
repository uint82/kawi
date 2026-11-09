use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::topdir::{ensure_directory, ensure_root, mount_point, same_device, user_id};
use super::{is_writable_location, set_private, unique_name, TrashError};

const TRASH_DIR: &str = "Trash";
const FILES_DIR: &str = "files";
const INFO_DIR: &str = "info";
const INFO_SUFFIX: &str = ".trashinfo";
const DATE_PREFIX: &str = "DeletionDate=";
const TOP_TRASH: &str = ".Trash";
const VOLUME_TRASH_PREFIX: &str = ".Trash-";

pub(super) struct FreedesktopTrash {
    root: PathBuf,
    topdir: Option<PathBuf>,
}

impl FreedesktopTrash {
    pub(super) fn for_target(target: &Path) -> Option<Self> {
        Self::from_env()
            .filter(|home| home.can_trash(target))
            .or_else(|| topdir_trash(target))
    }

    fn from_env() -> Option<Self> {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".local/share"))
            })?;
        Some(Self {
            root: data.join(TRASH_DIR),
            topdir: None,
        })
    }

    #[cfg(test)]
    pub(super) fn new(root: PathBuf) -> Self {
        Self { root, topdir: None }
    }

    #[cfg(test)]
    pub(super) fn with_topdir(root: PathBuf, topdir: PathBuf) -> Self {
        Self {
            root,
            topdir: Some(topdir),
        }
    }

    pub(super) fn can_trash(&self, target: &Path) -> bool {
        same_device(target, &self.root) && is_writable_location(&self.root)
    }

    pub(super) fn trash_as(&self, target: &Path, original: &Path) -> Result<PathBuf, TrashError> {
        self.ensure()?;
        if !same_device(target, &self.files_dir()) {
            return Err(TrashError::Unavailable(
                "trash is on another filesystem".to_string(),
            ));
        }
        let name = self.unique_name(original)?;
        let info = self.info_dir().join(format!("{name}{INFO_SUFFIX}"));
        write_info(&info, &self.info_path(original))?;
        let destination = self.files_dir().join(&name);
        match fs::rename(target, &destination) {
            Ok(()) => Ok(destination),
            Err(error) => {
                let _ = fs::remove_file(&info);
                Err(TrashError::Io(error))
            }
        }
    }

    fn ensure(&self) -> io::Result<()> {
        let _ = ensure_root(&self.root)?;
        ensure_directory(&self.files_dir())?;
        ensure_directory(&self.info_dir())?;
        set_private(&self.root)?;
        set_private(&self.files_dir())?;
        set_private(&self.info_dir())
    }

    fn files_dir(&self) -> PathBuf {
        self.root.join(FILES_DIR)
    }

    pub(super) fn info_dir(&self) -> PathBuf {
        self.root.join(INFO_DIR)
    }

    #[cfg(test)]
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn info_path(&self, target: &Path) -> String {
        let display = self
            .topdir
            .as_ref()
            .and_then(|topdir| target.strip_prefix(topdir).ok())
            .unwrap_or(target);
        encode_path(display)
    }

    fn unique_name(&self, target: &Path) -> io::Result<String> {
        let files = self.files_dir();
        let info = self.info_dir();
        unique_name(target, |name| {
            !files.join(name).exists() && !info.join(format!("{name}{INFO_SUFFIX}")).exists()
        })
    }
}

pub(super) fn topdir_trash(target: &Path) -> Option<FreedesktopTrash> {
    let topdir = mount_point(target)?;
    let shared = topdir.join(TOP_TRASH);
    if is_secure_shared_trash(&shared) {
        return Some(FreedesktopTrash {
            root: shared.join(user_id()?),
            topdir: Some(topdir),
        });
    }
    Some(FreedesktopTrash {
        root: topdir.join(format!("{VOLUME_TRASH_PREFIX}{}", user_id()?)),
        topdir: Some(topdir),
    })
}

fn is_secure_shared_trash(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o1000 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn write_info(path: &Path, encoded: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    writeln!(file, "[Trash Info]")?;
    writeln!(file, "Path={encoded}")?;
    writeln!(file, "{DATE_PREFIX}{}", deletion_date())?;
    file.sync_all()
}

fn encode_path(display: &Path) -> String {
    let bytes = path_bytes(display);
    let mut encoded = String::with_capacity(bytes.len());
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

fn hex_digit(value: u8) -> char {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    DIGITS[usize::from(value & 0x0f)] as char
}

pub(super) fn deletion_date() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let remainder = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    let second = remainder % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
}

pub(super) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}
