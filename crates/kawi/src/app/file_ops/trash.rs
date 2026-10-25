use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod topdir;

use topdir::{ensure_directory, ensure_root, same_device, topdir_trash};

const TRASH_DIR: &str = "Trash";
const FILES_DIR: &str = "files";
const INFO_DIR: &str = "info";
const INFO_SUFFIX: &str = ".trashinfo";
const DATE_PREFIX: &str = "DeletionDate=";
const TOP_TRASH: &str = ".Trash";

#[derive(Debug)]
pub(super) enum TrashError {
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

pub(super) struct TrashDir {
    root: PathBuf,
    topdir: Option<PathBuf>,
}

impl TrashDir {
    pub(super) fn for_target(target: &Path) -> Option<Self> {
        let home = Self::from_env();
        if let Some(home) = home {
            if same_device(target, &home.root) {
                return Some(home);
            }
        }
        topdir_trash(target)
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
    fn new(root: PathBuf) -> Self {
        Self { root, topdir: None }
    }

    pub(super) fn can_trash(&self, target: &Path) -> bool {
        same_device(target, &self.root) && is_writable_location(&self.root)
    }

    pub(super) fn trash(&self, target: &Path) -> Result<PathBuf, TrashError> {
        self.ensure()?;
        if !same_device(target, &self.files_dir()) {
            return Err(TrashError::Unavailable(
                "trash is on another filesystem".to_string(),
            ));
        }
        let name = self.unique_name(target)?;
        let info = self.info_dir().join(format!("{name}{INFO_SUFFIX}"));
        write_info(&info, &self.info_path(target))?;
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
        ensure_root(&self.root)?;
        ensure_directory(&self.files_dir())?;
        ensure_directory(&self.info_dir())?;
        set_private(&self.root)?;
        set_private(&self.files_dir())?;
        set_private(&self.info_dir())
    }

    fn files_dir(&self) -> PathBuf {
        self.root.join(FILES_DIR)
    }

    fn info_dir(&self) -> PathBuf {
        self.root.join(INFO_DIR)
    }

    fn info_path(&self, target: &Path) -> String {
        let display = self
            .topdir
            .as_ref()
            .and_then(|topdir| target.strip_prefix(topdir).ok())
            .unwrap_or(target);
        encode_path(display)
    }

    fn unique_name(&self, target: &Path) -> io::Result<String> {
        let base = target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "item".to_string());
        for index in 0..10_000 {
            let name = if index == 0 {
                base.clone()
            } else {
                format!("{base}.{index}")
            };
            let file = self.files_dir().join(&name);
            let info = self.info_dir().join(format!("{name}{INFO_SUFFIX}"));
            if !file.exists() && !info.exists() {
                return Ok(name);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "trash contains too many items with the same name",
        ))
    }
}

fn is_writable_location(path: &Path) -> bool {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                return metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                    && !metadata.permissions().readonly();
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return false,
        }
    }
    false
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

fn deletion_date() -> String {
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

fn civil_from_days(days: i64) -> (i64, u32, u32) {
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

#[cfg(unix)]
fn set_private(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
