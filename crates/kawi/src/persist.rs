use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn write_atomically(path: &Path, text: &str) -> Result<()> {
    let parent = path.parent().context("state file has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let temp = temp_path(path);
    let result = write_temp_file(&temp, text)
        .and_then(|()| {
            fs::rename(&temp, path).with_context(|| format!("failed to replace {}", path.display()))
        })
        .and_then(|()| sync_directory(parent));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn write_temp_file(path: &Path, text: &str) -> Result<()> {
    let mut file =
        File::create(path).with_context(|| format!("failed to write {}", path.display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", path.display()))
}

fn temp_path(path: &Path) -> PathBuf {
    path.with_extension(format!("{}.tmp", std::process::id()))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync {}", path.display()))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_persist_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn replaces_file_contents() {
        let dir = temp_dir("replace");
        let path = dir.join("state");
        write_atomically(&path, "first").unwrap();
        write_atomically(&path, "second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        assert!(!temp_path(&path).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn creates_parent_directories() {
        let dir = temp_dir("parents");
        let path = dir.join("nested").join("state");
        write_atomically(&path, "text").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "text");
        fs::remove_dir_all(dir).unwrap();
    }
}
