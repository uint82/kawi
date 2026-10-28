use std::env;
use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub fn find_executable(command: &str) -> Option<PathBuf> {
    find_executable_in(command, Path::new("."))
}

pub fn find_executable_in(command: &str, directory: &Path) -> Option<PathBuf> {
    if command.trim().is_empty() {
        return None;
    }
    let path = Path::new(command);
    if path.is_absolute() {
        return is_executable(path).then(|| path.to_path_buf());
    }
    if path.components().count() > 1 {
        let candidate = directory.join(path);
        return is_executable(&candidate).then_some(candidate);
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

#[cfg(unix)]
pub fn executable_interpreter(path: &Path) -> Option<PathBuf> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut ident = [0u8; 16];
    file.read_exact(&mut ident).ok()?;
    if &ident[..4] != b"\x7fELF" || ident[5] != 1 {
        return None;
    }
    match ident[4] {
        1 => elf32_interpreter(&mut file),
        2 => elf64_interpreter(&mut file),
        _ => None,
    }
}

#[cfg(not(unix))]
pub fn executable_interpreter(_path: &Path) -> Option<PathBuf> {
    None
}

#[cfg(unix)]
fn elf64_interpreter(file: &mut std::fs::File) -> Option<PathBuf> {
    use std::io::{Read, Seek, SeekFrom};
    let mut header = [0u8; 64];
    file.seek(SeekFrom::Start(0)).ok()?;
    file.read_exact(&mut header).ok()?;
    let phoff = u64::from_le_bytes(header[32..40].try_into().ok()?);
    let phentsize = u16::from_le_bytes(header[54..56].try_into().ok()?);
    let phnum = u16::from_le_bytes(header[56..58].try_into().ok()?);
    read_interpreter(file, phoff, phentsize, phnum, 8)
}

#[cfg(unix)]
fn elf32_interpreter(file: &mut std::fs::File) -> Option<PathBuf> {
    use std::io::{Read, Seek, SeekFrom};
    let mut header = [0u8; 52];
    file.seek(SeekFrom::Start(0)).ok()?;
    file.read_exact(&mut header).ok()?;
    let phoff = u64::from(u32::from_le_bytes(header[28..32].try_into().ok()?));
    let phentsize = u16::from_le_bytes(header[42..44].try_into().ok()?);
    let phnum = u16::from_le_bytes(header[44..46].try_into().ok()?);
    read_interpreter(file, phoff, phentsize, phnum, 4)
}

#[cfg(unix)]
fn read_interpreter(
    file: &mut std::fs::File,
    phoff: u64,
    phentsize: u16,
    phnum: u16,
    offset_size: usize,
) -> Option<PathBuf> {
    use std::io::{Read, Seek, SeekFrom};
    const PT_INTERP: u32 = 3;
    let mut entry = vec![0u8; phentsize as usize];
    for index in 0..phnum as u64 {
        file.seek(SeekFrom::Start(phoff + index * phentsize as u64))
            .ok()?;
        file.read_exact(&mut entry).ok()?;
        let kind = u32::from_le_bytes(entry[0..4].try_into().ok()?);
        if kind != PT_INTERP {
            continue;
        }
        let (offset, size) = match offset_size {
            8 => (
                u64::from_le_bytes(entry[8..16].try_into().ok()?),
                u64::from_le_bytes(entry[32..40].try_into().ok()?),
            ),
            _ => (
                u64::from(u32::from_le_bytes(entry[4..8].try_into().ok()?)),
                u64::from(u32::from_le_bytes(entry[16..20].try_into().ok()?)),
            ),
        };
        let mut bytes = vec![0u8; usize::try_from(size).ok()?];
        file.seek(SeekFrom::Start(offset)).ok()?;
        file.read_exact(&mut bytes).ok()?;
        let text = bytes.split(|byte| *byte == 0).next()?;
        return Some(PathBuf::from(std::ffi::OsStr::from_bytes(text)));
    }
    None
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
        let dir = std::env::temp_dir().join(format!("kawi_process_{name}_{stamp}_{id}"));
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
        assert_eq!(find_executable(file.to_str().unwrap()), Some(file));
    }

    #[test]
    fn resolves_a_relative_command_from_the_working_directory() {
        let dir = temp_dir("relative");
        let file = dir.join("server");
        executable(&file);
        assert_eq!(find_executable_in("./server", &dir), Some(file));
    }

    #[test]
    fn rejects_an_empty_command() {
        assert_eq!(find_executable(""), None);
    }
}
