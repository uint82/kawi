use std::fs;
use std::path::{Path, PathBuf};

use crate::persist::write_private_atomically;
use crate::recovery::directory::DirectoryJournal;

use super::WorkspaceStore;

const DIRECTORY_JOURNAL_NAME: &str = "directory-apply.json";

impl WorkspaceStore {
    pub(crate) fn save_directory_journal(&self, journal: &DirectoryJournal) -> Result<(), String> {
        let bytes = journal.to_bytes()?;
        write_private_atomically(&self.directory_journal_path(), &bytes)
            .map_err(|error| format!("directory journal: {error}"))
    }

    pub(crate) fn load_directory_journal(&self) -> Result<Option<DirectoryJournal>, String> {
        let path = self.directory_journal_path();
        match fs::read(&path) {
            Ok(bytes) => DirectoryJournal::from_bytes(&bytes)
                .map(Some)
                .map_err(|error| format!("{}: {error}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("directory journal: {error}")),
        }
    }

    pub(crate) fn remove_directory_journal(&self) -> Result<(), String> {
        remove_file(&self.directory_journal_path())
    }

    fn directory_journal_path(&self) -> PathBuf {
        self.directory.join(DIRECTORY_JOURNAL_NAME)
    }
}

fn remove_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("directory journal: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_directory_journal_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn journals_are_saved_loaded_and_removed() {
        let root = temp_dir("store");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let journal = DirectoryJournal::prepared(vec![]);
        store.save_directory_journal(&journal).unwrap();
        assert_eq!(store.load_directory_journal().unwrap(), Some(journal));
        store.remove_directory_journal().unwrap();
        assert_eq!(store.load_directory_journal().unwrap(), None);
        store.remove_directory_journal().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
