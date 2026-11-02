use crate::recovery::apply::{ApplyJournal, ApplyRecovery};
use crate::recovery::store::WorkspaceStore;

use super::RecoveryManager;

pub(super) fn load(store: &WorkspaceStore) -> (Option<ApplyRecovery>, Option<String>) {
    let journal = match store.load_apply_journal() {
        Ok(Some(journal)) => journal,
        Ok(None) => {
            return match store.remove_apply_promotions() {
                Ok(()) => (None, None),
                Err(error) => (None, Some(format!("apply recovery: {error}"))),
            };
        }
        Err(error) => return (None, Some(format!("apply recovery: {error}"))),
    };
    let (promotions, warning) = if journal.is_prepared() {
        match store.load_apply_promotions() {
            Ok(promotions) => (promotions, None),
            Err(error) => (Default::default(), Some(format!("apply recovery: {error}"))),
        }
    } else {
        (Default::default(), None)
    };
    (
        Some(ApplyRecovery {
            journal,
            promotions,
        }),
        warning,
    )
}

impl RecoveryManager {
    pub(crate) fn save_apply_journal(&self, journal: &ApplyJournal) -> Result<(), String> {
        self.store.save_apply_journal(journal)
    }

    pub(crate) fn remove_apply_journal(&self) -> Result<(), String> {
        self.store.remove_apply_journal()
    }

    pub(crate) fn remove_apply_promotions(&self) -> Result<(), String> {
        self.store.remove_apply_promotions()
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
            "kawi_apply_journal_manager_{}_{}",
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
        let prepared = ApplyJournal::prepared(vec![]);
        store.save_apply_journal(&prepared).unwrap();
        store.append_apply_promotion(1).unwrap();
        let state = load(&store).0.unwrap();
        assert_eq!(state.journal, prepared);
        assert_eq!(state.promotions, HashSet::from([1]));
        store
            .save_apply_journal(&prepared.clone().committed(Vec::new()))
            .unwrap();
        let state = load(&store).0.unwrap();
        assert!(!state.journal.is_prepared());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_promotions_without_a_journal_are_removed() {
        let root = temp_dir("stale_promotions");
        let store = WorkspaceStore::create(&root, &root).unwrap();
        store.append_apply_promotion(3).unwrap();
        assert!(load(&store).0.is_none());
        assert!(store.load_apply_promotions().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
