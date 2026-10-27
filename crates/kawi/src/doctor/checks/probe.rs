use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(1);

pub(super) struct Probe {
    pub(super) exists: bool,
    pub(super) writable: bool,
    pub(super) directory: PathBuf,
    pub(super) error: Option<String>,
}

pub(super) fn probe(path: &Path) -> Probe {
    let (directory, exists) = match find_existing_directory(path) {
        Ok(result) => result,
        Err(error) => {
            return Probe {
                exists: path.is_dir(),
                writable: false,
                directory: path.to_path_buf(),
                error: Some(error),
            }
        }
    };
    match probe_directory(&directory) {
        Ok(()) => Probe {
            exists,
            writable: true,
            directory,
            error: None,
        },
        Err(error) => Probe {
            exists,
            writable: false,
            directory,
            error: Some(error),
        },
    }
}

fn find_existing_directory(path: &Path) -> Result<(PathBuf, bool), String> {
    let mut current = path;
    loop {
        match fs::metadata(current) {
            Ok(metadata) if metadata.is_dir() => {
                return Ok((current.to_path_buf(), current == path))
            }
            Ok(_) => return Err(format!("{} is not a directory", current.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                current = current
                    .parent()
                    .ok_or_else(|| format!("no existing parent for {}", path.display()))?;
            }
            Err(error) => return Err(format!("{}: {error}", current.display())),
        }
    }
}

fn probe_directory(directory: &Path) -> Result<(), String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let path = directory.join(format!(
        ".kawi-doctor-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let write = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| file.write_all(b"kawi\n"));
    let remove = fs::remove_file(&path);
    if let Err(error) = write {
        return Err(format!("{}: {error}", directory.display()));
    }
    if let Err(error) = remove {
        return Err(format!("{}: {error}", directory.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_doctor_probe_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn probes_an_existing_directory_without_leaving_files() {
        let dir = temp_dir("existing");
        let result = probe(&dir);
        assert!(result.exists);
        assert!(result.writable);
        assert_eq!(result.error, None);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn probes_the_parent_of_a_missing_directory() {
        let dir = temp_dir("missing");
        let missing = dir.join("sessions").join("nested");
        let result = probe(&missing);
        assert!(!result.exists);
        assert!(result.writable);
        assert_eq!(result.directory, dir);
        assert!(!missing.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_a_file_as_a_state_directory() {
        let dir = temp_dir("file");
        let file = dir.join("state");
        fs::write(&file, "not a directory").unwrap();
        let result = probe(&file);
        assert!(!result.writable);
        assert!(result.error.unwrap().contains("not a directory"));
        fs::remove_dir_all(dir).unwrap();
    }
}
