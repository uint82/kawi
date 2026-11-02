use crate::recovery::directory::DirectoryJournal;
use crate::recovery::store::WorkspaceStore;

use super::RecoveryManager;

pub(super) fn load(store: &WorkspaceStore) -> (Option<DirectoryJournal>, Option<String>) {
    let journal = match store.load_directory_journal() {
        Ok(Some(journal)) => journal,
        Ok(None) => return (None, None),
        Err(error) => return (None, Some(format!("directory recovery: {error}"))),
    };
    if journal.is_prepared() {
        return (Some(journal), None);
    }
    match store.remove_directory_journal() {
        Ok(()) => (None, None),
        Err(error) => (
            None,
            Some(format!(
                "directory recovery: cannot remove journal: {error}"
            )),
        ),
    }
}

impl RecoveryManager {
    pub(crate) fn save_directory_journal(&self, journal: &DirectoryJournal) -> Result<(), String> {
        self.store.save_directory_journal(journal)
    }

    pub(crate) fn remove_directory_journal(&self) -> Result<(), String> {
        self.store.remove_directory_journal()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_directory_journal_manager_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prepared_journals_are_loaded_and_committed_journals_are_removed() {
        let root = temp_dir("phases");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let prepared = DirectoryJournal::prepared(vec![]);
        store.save_directory_journal(&prepared).unwrap();
        assert_eq!(load(&store).0, Some(prepared.clone()));
        store
            .save_directory_journal(&prepared.clone().committed())
            .unwrap();
        assert_eq!(load(&store).0, None);
        assert_eq!(store.load_directory_journal().unwrap(), None);
        fs::remove_dir_all(root).unwrap();
    }
}
