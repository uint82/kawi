use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

mod path;

pub(crate) use path::JournalPath;

pub(crate) const APPLY_JOURNAL_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum JournalPhase {
    Prepared,
    Committed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApplyRecovery {
    pub journal: ApplyJournal,
    pub promotions: HashSet<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ApplyJournal {
    pub version: u32,
    pub phase: JournalPhase,
    pub operations: Vec<JournalOperation>,
    #[serde(default)]
    pub deferred: Vec<JournalPath>,
    #[serde(default)]
    pub cleanup: Vec<JournalCleanup>,
}

impl ApplyJournal {
    pub(crate) fn prepared(operations: Vec<JournalOperation>) -> Self {
        Self {
            version: APPLY_JOURNAL_VERSION,
            phase: JournalPhase::Prepared,
            operations,
            deferred: Vec::new(),
            cleanup: Vec::new(),
        }
    }

    pub(crate) fn committed(mut self, cleanup: Vec<JournalCleanup>) -> Self {
        self.phase = JournalPhase::Committed;
        self.cleanup = cleanup;
        self
    }

    pub(crate) fn is_prepared(&self) -> bool {
        self.phase == JournalPhase::Prepared
    }

    pub(crate) fn cleanups(&self) -> Vec<JournalCleanup> {
        let mut cleanups = self.cleanup.clone();
        cleanups.extend(
            self.deferred
                .iter()
                .cloned()
                .map(|path| JournalCleanup::Remove { path }),
        );
        cleanups
    }

    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|error| format!("apply journal: {error}"))
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let journal: Self =
            serde_json::from_slice(bytes).map_err(|error| format!("apply journal: {error}"))?;
        validate(&journal)?;
        Ok(journal)
    }
}

fn validate(journal: &ApplyJournal) -> Result<(), String> {
    if journal.version != APPLY_JOURNAL_VERSION {
        return Err(format!(
            "unsupported apply journal version {}",
            journal.version
        ));
    }
    let operation_paths = journal.operations.iter().flat_map(JournalOperation::paths);
    let cleanup_paths = journal
        .cleanup
        .iter()
        .flat_map(JournalCleanup::paths)
        .chain(journal.deferred.iter().map(JournalPath::as_path));
    if operation_paths
        .chain(cleanup_paths)
        .any(|path| !path.is_absolute())
    {
        return Err("apply journal contains a relative path".to_string());
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum JournalOperation {
    Rename {
        from: JournalPath,
        to: JournalPath,
        #[serde(default)]
        stage: Option<JournalPath>,
        #[serde(default)]
        force: bool,
    },
    Copy {
        from: JournalPath,
        to: JournalPath,
        #[serde(default)]
        stage: Option<JournalPath>,
    },
    CreateDirectory {
        path: JournalPath,
    },
    CreateFile {
        path: JournalPath,
    },
    Delete {
        path: JournalPath,
        staged: JournalPath,
    },
}

impl JournalOperation {
    fn paths(&self) -> impl Iterator<Item = &Path> {
        let (first, second, stage) = match self {
            Self::Rename {
                from, to, stage, ..
            }
            | Self::Copy { from, to, stage } => (Some(from), Some(to), stage.as_ref()),
            Self::CreateDirectory { path } | Self::CreateFile { path } => (Some(path), None, None),
            Self::Delete { path, staged } => (Some(path), Some(staged), None),
        };
        first
            .into_iter()
            .chain(second)
            .chain(stage)
            .map(|path| path.as_path())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum JournalCleanup {
    Remove {
        path: JournalPath,
    },
    Trash {
        staged: JournalPath,
        original: JournalPath,
    },
}

impl JournalCleanup {
    pub(crate) fn remove(path: PathBuf) -> Self {
        Self::Remove {
            path: JournalPath::from(path),
        }
    }

    pub(crate) fn trash(staged: PathBuf, original: PathBuf) -> Self {
        Self::Trash {
            staged: JournalPath::from(staged),
            original: JournalPath::from(original),
        }
    }

    fn paths(&self) -> impl Iterator<Item = &Path> {
        let (first, second) = match self {
            Self::Remove { path } => (path, None),
            Self::Trash { staged, original } => (staged, Some(original)),
        };
        std::iter::once(first.as_path()).chain(second.map(JournalPath::as_path))
    }
}

#[cfg(test)]
mod tests;
