use crate::recovery::directory::{DirectoryJournal, DirectoryRecovery};
use crate::recovery::store::WorkspaceStore;

use super::RecoveryManager;

pub(super) fn load(store: &WorkspaceStore) -> (Option<DirectoryRecovery>, Option<String>) {
    let journal = match store.load_directory_journal() {
        Ok(Some(journal)) => journal,
        Ok(None) => {
            return match store.remove_directory_promotions() {
                Ok(()) => (None, None),
                Err(error) => (None, Some(format!("directory recovery: {error}"))),
            };
        }
        Err(error) => return (None, Some(format!("directory recovery: {error}"))),
    };
    let (promotions, warning) = if journal.is_prepared() {
        match store.load_directory_promotions() {
            Ok(promotions) => (promotions, None),
            Err(error) => (
                Default::default(),
                Some(format!("directory recovery: {error}")),
            ),
        }
    } else {
        (Default::default(), None)
    };
    (
        Some(DirectoryRecovery {
            journal,
            promotions,
        }),
        warning,
    )
}

impl RecoveryManager {
    pub(crate) fn save_directory_journal(&self, journal: &DirectoryJournal) -> Result<(), String> {
        self.store.save_directory_journal(journal)
    }

    pub(crate) fn remove_directory_journal(&self) -> Result<(), String> {
        self.store.remove_directory_journal()
    }

    pub(crate) fn remove_directory_promotions(&self) -> Result<(), String> {
        self.store.remove_directory_promotions()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
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
    fn prepared_and_committed_journals_are_loaded_with_promotions() {
        let root = temp_dir("phases");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        let prepared = DirectoryJournal::prepared(vec![]);
        store.save_directory_journal(&prepared).unwrap();
        store.append_directory_promotion(1).unwrap();
        let state = load(&store).0.unwrap();
        assert_eq!(state.journal, prepared);
        assert_eq!(state.promotions, HashSet::from([1]));
        store
            .save_directory_journal(&prepared.clone().committed(Vec::new()))
            .unwrap();
        let state = load(&store).0.unwrap();
        assert!(!state.journal.is_prepared());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_promotions_without_a_journal_are_removed() {
        let root = temp_dir("stale_promotions");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        store.append_directory_promotion(3).unwrap();
        assert!(load(&store).0.is_none());
        assert!(store.load_directory_promotions().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
