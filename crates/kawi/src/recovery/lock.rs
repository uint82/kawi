use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use anyhow::{Context, Result};

const LOCK_NAME: &str = "owner.lock";

#[derive(Debug)]
pub(crate) enum LockState {
    Acquired(Lock),
    Busy(u32),
}

#[derive(Debug)]
pub(crate) struct Lock {
    path: PathBuf,
    owned: bool,
}

impl Lock {
    pub(crate) fn release(&mut self) {
        if !self.owned {
            return;
        }
        let _ = fs::remove_file(&self.path);
        self.owned = false;
    }

    pub(crate) fn keep(&mut self) {
        self.owned = false;
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        self.release();
    }
}

pub(crate) fn acquire(directory: &Path) -> Result<LockState> {
    let path = directory.join(LOCK_NAME);
    for _ in 0..3 {
        match create(&path) {
            Ok(Some(())) => {
                return Ok(LockState::Acquired(Lock { path, owned: true }));
            }
            Ok(None) => {}
            Err(error) => return Err(error),
        }
        let Some(owner) = read_owner(&path)? else {
            remove_lock(&path)?;
            continue;
        };
        if is_alive(owner) {
            return Ok(LockState::Busy(owner));
        }
        remove_lock(&path)?;
    }
    anyhow::bail!("could not acquire the workspace recovery lock")
}

fn create(path: &Path) -> Result<Option<()>> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    apply_mode(&mut options);
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("create {}", path.display())),
    };
    writeln!(file, "{}", std::process::id())
        .with_context(|| format!("write {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("sync {}", path.display()))?;
    Ok(Some(()))
}

fn read_owner(path: &Path) -> Result<Option<u32>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text.trim().parse().ok()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn remove_lock(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", path.display())),
    }
}

#[cfg(unix)]
fn apply_mode(options: &mut OpenOptions) {
    options.mode(0o600);
}

#[cfg(not(unix))]
fn apply_mode(_options: &mut OpenOptions) {}

#[cfg(unix)]
pub(crate) fn is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
pub(crate) fn is_alive(pid: u32) -> bool {
    pid != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_recovery_lock_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn acquired(state: LockState) -> Lock {
        match state {
            LockState::Acquired(lock) => lock,
            LockState::Busy(pid) => panic!("unexpected busy lock for {pid}"),
        }
    }

    #[test]
    fn live_locks_are_reported_busy() {
        let dir = temp_dir("busy");
        let lock = acquired(acquire(&dir).unwrap());
        match acquire(&dir).unwrap() {
            LockState::Busy(pid) => assert_eq!(pid, std::process::id()),
            LockState::Acquired(_) => panic!("expected a busy lock"),
        }
        drop(lock);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn released_locks_can_be_reacquired() {
        let dir = temp_dir("release");
        let mut lock = acquired(acquire(&dir).unwrap());
        lock.release();
        drop(acquired(acquire(&dir).unwrap()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn dead_locks_are_replaced() {
        let dir = temp_dir("dead");
        fs::write(dir.join(LOCK_NAME), "0\n").unwrap();
        drop(acquired(acquire(&dir).unwrap()));
        assert!(!dir.join(LOCK_NAME).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn kept_locks_survive_drop() {
        let dir = temp_dir("kept");
        let mut lock = acquired(acquire(&dir).unwrap());
        lock.keep();
        drop(lock);
        assert!(dir.join(LOCK_NAME).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_current_process_is_alive() {
        assert!(is_alive(std::process::id()));
        assert!(!is_alive(0));
    }

    #[test]
    fn lock_files_are_private() {
        let dir = temp_dir("permissions");
        let lock = acquired(acquire(&dir).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join(LOCK_NAME))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
        drop(lock);
        fs::remove_dir_all(dir).unwrap();
    }
}
