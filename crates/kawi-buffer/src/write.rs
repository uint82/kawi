use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub fn write(
    path: &Path,
    force: bool,
    produce: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    ensure_not_directory(path)?;
    ensure_writable(path, force)?;
    if keep_inode(path) {
        write_in_place(path, produce)
    } else {
        write_atomic(path, produce)
    }
}

fn ensure_not_directory(path: &Path) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Err(directory_error(path)),
        _ => Ok(()),
    }
}

fn ensure_writable(path: &Path, force: bool) -> io::Result<()> {
    if force {
        return Ok(());
    }
    match fs::metadata(path) {
        Ok(meta) if !writable(&meta) => Err(readonly_error()),
        _ => Ok(()),
    }
}

fn writable(meta: &Metadata) -> bool {
    #[cfg(unix)]
    {
        meta.mode() & 0o200 != 0
    }
    #[cfg(not(unix))]
    {
        !meta.permissions().readonly()
    }
}

fn keep_inode(path: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    meta.file_type().is_symlink() || !plain_file(&meta)
}

fn plain_file(meta: &Metadata) -> bool {
    meta.is_file() && hard_links(meta) == 1
}

fn hard_links(meta: &Metadata) -> u64 {
    #[cfg(unix)]
    {
        meta.nlink()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        1
    }
}

fn directory_error(path: &Path) -> io::Error {
    io::Error::other(format!("E502: \"{}\" is a directory", path.display()))
}

fn readonly_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "E45: 'readonly' option is set (add ! to override)",
    )
}

fn write_atomic(
    path: &Path,
    produce: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    let temp_path = temp_path_for(path);
    let Ok(mut temp) = create_temp(&temp_path) else {
        return write_in_place(path, produce);
    };
    let original = fs::metadata(path).ok();
    if let Some(meta) = &original {
        if !same_owner(&temp, meta) {
            drop(temp);
            let _ = fs::remove_file(&temp_path);
            return write_in_place(path, produce);
        }
    }
    let result = publish_temp(path, &temp_path, &mut temp, original.as_ref(), produce);
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn create_temp(temp_path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
}

fn publish_temp(
    path: &Path,
    temp_path: &Path,
    temp: &mut File,
    original: Option<&Metadata>,
    produce: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    produce(temp)?;
    temp.sync_all()?;
    if let Some(meta) = original {
        fs::set_permissions(temp_path, meta.permissions())?;
    }
    fs::rename(temp_path, path)?;
    sync_directory(path);
    Ok(())
}

fn same_owner(file: &File, other: &Metadata) -> bool {
    let Ok(meta) = file.metadata() else {
        return false;
    };
    #[cfg(unix)]
    {
        meta.uid() == other.uid() && meta.gid() == other.gid()
    }
    #[cfg(not(unix))]
    {
        let _ = (meta, other);
        true
    }
}

fn write_in_place(
    path: &Path,
    produce: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    let truncate = regular_target(path);
    let mut file = open_in_place(path)?;
    let backup = truncate.then(|| copy_backup(path)).flatten();
    match write_through(&mut file, truncate, produce) {
        Ok(()) => {
            discard_backup(backup.as_deref());
            Ok(())
        }
        Err(error) => {
            restore_backup(backup.as_deref(), path)?;
            Err(error)
        }
    }
}

fn regular_target(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(true)
}

fn open_in_place(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

fn write_through(
    file: &mut File,
    truncate: bool,
    produce: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    if truncate {
        file.set_len(0)?;
    }
    produce(file)?;
    file.sync_all()
}

fn copy_backup(path: &Path) -> Option<PathBuf> {
    let backup = backup_path_for(path);
    fs::copy(path, &backup).ok()?;
    Some(backup)
}

fn discard_backup(backup: Option<&Path>) {
    if let Some(path) = backup {
        let _ = fs::remove_file(path);
    }
}

fn restore_backup(backup: Option<&Path>, path: &Path) -> io::Result<()> {
    let Some(backup) = backup else {
        return Ok(());
    };
    fs::rename(backup, path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("{error}: original kept at {}", backup.display()),
        )
    })
}

fn temp_path_for(path: &Path) -> PathBuf {
    sibling(path, "tmp")
}

fn backup_path_for(path: &Path) -> PathBuf {
    sibling(path, "bak")
}

fn sibling(path: &Path, role: &str) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "buffer".to_string());
    path.with_file_name(format!(".{name}.{}.kawi.{role}", std::process::id()))
}

fn sync_directory(path: &Path) {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let _ = File::open(parent).and_then(|dir| dir.sync_all());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::Permissions;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn case_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kawi_safe_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create case dir");
        dir
    }

    fn written(content: &str) -> impl FnOnce(&mut dyn Write) -> io::Result<()> + '_ {
        move |out| out.write_all(content.as_bytes())
    }

    fn failing() -> impl FnOnce(&mut dyn Write) -> io::Result<()> {
        |_| Err(io::Error::other("produce failed"))
    }

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).expect("stat").permissions().mode() & 0o7777
    }

    fn set_mode(path: &Path, bits: u32) {
        fs::set_permissions(path, Permissions::from_mode(bits)).expect("chmod");
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("read dir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|entry| entry.contains(".kawi."))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_new_file() {
        let dir = case_dir("new");
        let target = dir.join("fresh.txt");
        write(&target, false, written("hello\n")).expect("write");
        assert_eq!(fs::read_to_string(&target).expect("read"), "hello\n");
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn replaces_content_and_keeps_mode() {
        let dir = case_dir("replace");
        let target = dir.join("file.txt");
        fs::write(&target, "old\n").expect("seed");
        set_mode(&target, 0o640);
        write(&target, false, written("new\n")).expect("write");
        assert_eq!(fs::read_to_string(&target).expect("read"), "new\n");
        assert_eq!(mode(&target), 0o640);
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn follows_symlink() {
        let dir = case_dir("symlink");
        let target = dir.join("real.txt");
        let link = dir.join("link.txt");
        fs::write(&target, "old\n").expect("seed");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        write(&link, false, written("new\n")).expect("write");
        assert!(fs::symlink_metadata(&link)
            .expect("stat link")
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).expect("read"), "new\n");
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn writes_dangling_symlink_target() {
        let dir = case_dir("dangling");
        let target = dir.join("missing.txt");
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        write(&link, false, written("new\n")).expect("write");
        assert!(fs::symlink_metadata(&link)
            .expect("stat link")
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).expect("read"), "new\n");
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn keeps_hard_link() {
        let dir = case_dir("hardlink");
        let target = dir.join("one.txt");
        let other = dir.join("two.txt");
        fs::write(&target, "old\n").expect("seed");
        fs::hard_link(&target, &other).expect("hard link");
        write(&target, false, written("new\n")).expect("write");
        assert_eq!(fs::read_to_string(&target).expect("read"), "new\n");
        assert_eq!(fs::read_to_string(&other).expect("read"), "new\n");
        assert_eq!(hard_links(&fs::metadata(&target).expect("stat")), 2);
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn rejects_readonly_file() {
        let dir = case_dir("readonly");
        let target = dir.join("ro.txt");
        fs::write(&target, "old\n").expect("seed");
        set_mode(&target, 0o444);
        let error = write(&target, false, written("new\n")).expect_err("readonly");
        assert!(error.to_string().starts_with("E45"), "{error}");
        assert_eq!(fs::read_to_string(&target).expect("read"), "old\n");
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn forced_write_keeps_mode() {
        let dir = case_dir("forced");
        let target = dir.join("ro.txt");
        fs::write(&target, "old\n").expect("seed");
        set_mode(&target, 0o444);
        write(&target, true, written("new\n")).expect("forced write");
        assert_eq!(fs::read_to_string(&target).expect("read"), "new\n");
        assert_eq!(mode(&target), 0o444);
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn rejects_directory() {
        let dir = case_dir("directory");
        let target = dir.join("sub");
        fs::create_dir(&target).expect("mkdir");
        let error = write(&target, false, written("new\n")).expect_err("directory");
        assert!(error.to_string().starts_with("E502"), "{error}");
    }

    #[test]
    fn writes_in_place_when_directory_readonly() {
        let dir = case_dir("readonly_dir");
        let target = dir.join("file.txt");
        fs::write(&target, "old\n").expect("seed");
        set_mode(&dir, 0o555);
        let result = write(&target, false, written("new\n"));
        set_mode(&dir, 0o755);
        result.expect("in place write");
        assert_eq!(fs::read_to_string(&target).expect("read"), "new\n");
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn keeps_original_when_produce_fails() {
        let dir = case_dir("produce_fail");
        let target = dir.join("file.txt");
        fs::write(&target, "old\n").expect("seed");
        write(&target, false, failing()).expect_err("produce failure");
        assert_eq!(fs::read_to_string(&target).expect("read"), "old\n");
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn restores_original_when_in_place_fails() {
        let dir = case_dir("in_place_fail");
        let target = dir.join("one.txt");
        let other = dir.join("two.txt");
        fs::write(&target, "old\n").expect("seed");
        fs::hard_link(&target, &other).expect("hard link");
        write(&target, false, failing()).expect_err("produce failure");
        assert_eq!(fs::read_to_string(&target).expect("read"), "old\n");
        assert!(leftovers(&dir).is_empty());
    }
}
