use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::GrammarError;
use crate::limits;
use crate::pack::Pack;
use crate::store::{InstalledPack, PackRoot, PackRootKind};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn install(roots: &[PackRoot], source: &Path) -> Result<InstalledPack, GrammarError> {
    let pack = Pack::open(source)?;
    pack.verify()?;
    let user_root = roots
        .iter()
        .find(|root| root.kind == PackRootKind::User)
        .ok_or_else(|| GrammarError::Install("no user grammar root".to_string()))?;
    let parent = user_root.path.join(&pack.manifest().id);
    fs::create_dir_all(&parent).map_err(|source| GrammarError::io(&parent, source))?;
    let mut temp = TempDir::create(&parent)?;
    copy_tree(source, temp.path(), &mut Budget::new())?;
    let copied = Pack::open(temp.path())?;
    copied.verify()?;
    let target = parent.join(&copied.manifest().version);
    replace_target(temp.path(), &target)?;
    temp.keep();
    Ok(InstalledPack {
        id: copied.manifest().id.clone(),
        version: copied.manifest().version.clone(),
        path: target,
    })
}

fn replace_target(temp: &Path, target: &Path) -> Result<(), GrammarError> {
    if !target.exists() {
        return fs::rename(temp, target).map_err(|source| GrammarError::io(target, source));
    }
    let backup = target.with_file_name(format!(".backup-{}", unique_suffix()));
    fs::rename(target, &backup).map_err(|source| GrammarError::io(&backup, source))?;
    match fs::rename(temp, target) {
        Ok(()) => {
            remove_path(&backup).map_err(|source| GrammarError::io(&backup, source))?;
            Ok(())
        }
        Err(source) => {
            if let Err(restore) = fs::rename(&backup, target) {
                return Err(GrammarError::Install(format!(
                    "cannot replace {}: {source}; the previous pack is at {} and could not be restored: {restore}",
                    target.display(),
                    backup.display()
                )));
            }
            Err(GrammarError::io(target, source))
        }
    }
}

struct TempDir {
    path: PathBuf,
    keep: bool,
}

impl TempDir {
    fn create(parent: &Path) -> Result<Self, GrammarError> {
        for _ in 0..16 {
            let path = parent.join(format!(".install-{}", unique_suffix()));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self { path, keep: false });
                }
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(GrammarError::io(&path, source)),
            }
        }
        Err(GrammarError::Install(
            "could not create a temporary install directory".to_string(),
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.keep {
            let _ = remove_path(&self.path);
        }
    }
}

struct Budget {
    bytes: u64,
    entries: usize,
}

impl Budget {
    fn new() -> Self {
        Self {
            bytes: 0,
            entries: 0,
        }
    }

    fn add(&mut self, path: &Path, size: u64) -> Result<(), GrammarError> {
        self.entries += 1;
        if self.entries > limits::MAX_PACK_FILES {
            return Err(GrammarError::Install(format!(
                "pack exceeds the {} entry limit at {}",
                limits::MAX_PACK_FILES,
                path.display()
            )));
        }
        self.bytes = self.bytes.saturating_add(size);
        if self.bytes > limits::MAX_PACK_BYTES {
            return Err(GrammarError::Install(format!(
                "pack exceeds the {} byte limit at {}",
                limits::MAX_PACK_BYTES,
                path.display()
            )));
        }
        Ok(())
    }
}

fn copy_tree(source: &Path, target: &Path, budget: &mut Budget) -> Result<(), GrammarError> {
    copy_tree_at(source, target, budget, 0)
}

fn copy_tree_at(
    source: &Path,
    target: &Path,
    budget: &mut Budget,
    depth: usize,
) -> Result<(), GrammarError> {
    if depth > limits::MAX_PACK_DEPTH {
        return Err(GrammarError::Install(format!(
            "pack nesting exceeds {} levels at {}",
            limits::MAX_PACK_DEPTH,
            source.display()
        )));
    }
    let entries = fs::read_dir(source).map_err(|error| GrammarError::io(source, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| GrammarError::io(source, error))?;
        copy_entry(&entry, target, budget, depth)?;
    }
    Ok(())
}

fn copy_entry(
    entry: &fs::DirEntry,
    target: &Path,
    budget: &mut Budget,
    depth: usize,
) -> Result<(), GrammarError> {
    let path = entry.path();
    let destination = target.join(entry.file_name());
    let file_type = entry
        .file_type()
        .map_err(|error| GrammarError::io(&path, error))?;
    if file_type.is_symlink() {
        return Err(GrammarError::Symlink(path));
    }
    if file_type.is_dir() {
        return copy_dir(&path, &destination, budget, depth);
    }
    if file_type.is_file() {
        return copy_file(&path, &destination, budget);
    }
    Err(GrammarError::UnsupportedFileType(path))
}

fn copy_dir(
    path: &Path,
    destination: &Path,
    budget: &mut Budget,
    depth: usize,
) -> Result<(), GrammarError> {
    budget.add(path, 0)?;
    fs::create_dir(destination).map_err(|error| GrammarError::io(destination, error))?;
    copy_tree_at(path, destination, budget, depth + 1)
}

fn copy_file(path: &Path, destination: &Path, budget: &mut Budget) -> Result<(), GrammarError> {
    let metadata = fs::metadata(path).map_err(|error| GrammarError::io(path, error))?;
    budget.add(path, metadata.len())?;
    fs::copy(path, destination).map_err(|error| GrammarError::io(destination, error))?;
    Ok(())
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{nanos}-{counter}", process::id())
}

fn remove_path(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
