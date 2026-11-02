use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::persist::write_private_atomically;
use crate::recovery::apply::ApplyJournal;

use super::WorkspaceStore;

const APPLY_JOURNAL_NAME: &str = "apply-journal.json";
const APPLY_PROMOTIONS_NAME: &str = "apply-promotions.log";
const LEGACY_APPLY_JOURNAL_NAME: &str = "directory-apply.json";
const LEGACY_APPLY_PROMOTIONS_NAME: &str = "directory-promotions.log";

impl WorkspaceStore {
    pub(crate) fn save_apply_journal(&self, journal: &ApplyJournal) -> Result<(), String> {
        let bytes = journal.to_bytes()?;
        write_private_atomically(&self.apply_journal_path(), &bytes)
            .map_err(|error| format!("apply journal: {error}"))
    }

    pub(crate) fn load_apply_journal(&self) -> Result<Option<ApplyJournal>, String> {
        let Some(path) = self.existing_journal_path() else {
            return Ok(None);
        };
        match fs::read(&path) {
            Ok(bytes) => ApplyJournal::from_bytes(&bytes)
                .map(Some)
                .map_err(|error| format!("{}: {error}", path.display())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("apply journal: {error}")),
        }
    }

    pub(crate) fn remove_apply_journal(&self) -> Result<(), String> {
        remove_file(&self.apply_journal_path(), "apply journal")?;
        remove_file(&self.legacy_apply_journal_path(), "apply journal")
    }

    pub(crate) fn append_apply_promotion(&self, operation: usize) -> Result<(), String> {
        let mut file = append_file(&self.apply_promotions_path())
            .map_err(|error| format!("apply promotions: {error}"))?;
        writeln!(file, "{operation}")
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("apply promotions: {error}"))
    }

    pub(crate) fn load_apply_promotions(&self) -> Result<HashSet<usize>, String> {
        let Some(path) = self.existing_promotions_path() else {
            return Ok(HashSet::new());
        };
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(HashSet::new()),
            Err(error) => return Err(format!("apply promotions: {error}")),
        };
        parse_promotions(&path, &text)
    }

    pub(crate) fn remove_apply_promotions(&self) -> Result<(), String> {
        remove_file(&self.apply_promotions_path(), "apply promotions")?;
        remove_file(&self.legacy_apply_promotions_path(), "apply promotions")
    }

    fn apply_journal_path(&self) -> PathBuf {
        self.directory.join(APPLY_JOURNAL_NAME)
    }

    fn apply_promotions_path(&self) -> PathBuf {
        self.directory.join(APPLY_PROMOTIONS_NAME)
    }

    fn legacy_apply_journal_path(&self) -> PathBuf {
        self.directory.join(LEGACY_APPLY_JOURNAL_NAME)
    }

    fn legacy_apply_promotions_path(&self) -> PathBuf {
        self.directory.join(LEGACY_APPLY_PROMOTIONS_NAME)
    }

    fn existing_journal_path(&self) -> Option<PathBuf> {
        existing_path(
            &self.apply_journal_path(),
            &self.legacy_apply_journal_path(),
        )
    }

    fn existing_promotions_path(&self) -> Option<PathBuf> {
        existing_path(
            &self.apply_promotions_path(),
            &self.legacy_apply_promotions_path(),
        )
    }
}

fn existing_path(current: &Path, legacy: &Path) -> Option<PathBuf> {
    if current.is_file() {
        Some(current.to_path_buf())
    } else if legacy.is_file() {
        Some(legacy.to_path_buf())
    } else {
        None
    }
}

fn parse_promotions(path: &Path, text: &str) -> Result<HashSet<usize>, String> {
    text.lines()
        .map(|line| {
            line.parse::<usize>()
                .map_err(|error| format!("{}: invalid promotion record: {error}", path.display()))
        })
        .collect()
}

fn append_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn remove_file(path: &Path, label: &str) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{label}: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_apply_journal_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn promotions_are_appended_loaded_and_removed() {
        let root = temp_dir("promotions");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        store.append_apply_promotion(2).unwrap();
        store.append_apply_promotion(5).unwrap();
        assert_eq!(
            store.load_apply_promotions().unwrap(),
            HashSet::from([2, 5])
        );
        store.remove_apply_promotions().unwrap();
        assert!(store.load_apply_promotions().unwrap().is_empty());
        store.remove_apply_promotions().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_journals_are_loaded_and_removed() {
        let root = temp_dir("legacy");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let journal = ApplyJournal::prepared(vec![]);
        let bytes = journal.to_bytes().unwrap();
        fs::write(store.legacy_apply_journal_path(), bytes).unwrap();
        fs::write(store.legacy_apply_promotions_path(), "2\n").unwrap();
        assert_eq!(store.load_apply_journal().unwrap(), Some(journal));
        assert_eq!(store.load_apply_promotions().unwrap(), HashSet::from([2]));
        store.remove_apply_journal().unwrap();
        store.remove_apply_promotions().unwrap();
        assert!(store.load_apply_journal().unwrap().is_none());
        assert!(store.load_apply_promotions().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn journals_are_saved_loaded_and_removed() {
        let root = temp_dir("store");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let journal = ApplyJournal::prepared(vec![]);
        store.save_apply_journal(&journal).unwrap();
        assert_eq!(store.load_apply_journal().unwrap(), Some(journal));
        store.remove_apply_journal().unwrap();
        assert_eq!(store.load_apply_journal().unwrap(), None);
        store.remove_apply_journal().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
