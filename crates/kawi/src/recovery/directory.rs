use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub(crate) const DIRECTORY_JOURNAL_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum JournalPhase {
    Prepared,
    Committed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DirectoryRecovery {
    pub journal: DirectoryJournal,
    pub promotions: HashSet<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DirectoryJournal {
    pub version: u32,
    pub phase: JournalPhase,
    pub operations: Vec<JournalOperation>,
    #[serde(default)]
    pub deferred: Vec<JournalPath>,
}

impl DirectoryJournal {
    pub(crate) fn prepared(operations: Vec<JournalOperation>) -> Self {
        Self {
            version: DIRECTORY_JOURNAL_VERSION,
            phase: JournalPhase::Prepared,
            operations,
            deferred: Vec::new(),
        }
    }

    pub(crate) fn committed(mut self, deferred: Vec<JournalPath>) -> Self {
        self.phase = JournalPhase::Committed;
        self.deferred = deferred;
        self
    }

    pub(crate) fn is_prepared(&self) -> bool {
        self.phase == JournalPhase::Prepared
    }

    pub(crate) fn deferred(&self) -> &[JournalPath] {
        &self.deferred
    }

    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|error| format!("directory journal: {error}"))
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let journal: Self =
            serde_json::from_slice(bytes).map_err(|error| format!("directory journal: {error}"))?;
        validate(&journal)?;
        Ok(journal)
    }
}

fn validate(journal: &DirectoryJournal) -> Result<(), String> {
    if journal.version != DIRECTORY_JOURNAL_VERSION {
        return Err(format!(
            "unsupported directory journal version {}",
            journal.version
        ));
    }
    let operation_paths = journal.operations.iter().flat_map(JournalOperation::paths);
    if operation_paths
        .chain(journal.deferred.iter().map(JournalPath::as_path))
        .any(|path| !path.is_absolute())
    {
        return Err("directory journal contains a relative path".to_string());
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
            Self::Rename { from, to, stage } | Self::Copy { from, to, stage } => {
                (Some(from), Some(to), stage.as_ref())
            }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JournalPath(PathBuf);

impl JournalPath {
    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

impl From<PathBuf> for JournalPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Serialize for JournalPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(text) = self.0.to_str() {
            return serializer.serialize_str(text);
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            serializer.serialize_bytes(self.0.as_os_str().as_bytes())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = self.0.as_os_str().encode_wide().collect();
            wide.serialize(serializer)
        }
        #[cfg(not(any(unix, windows)))]
        {
            self.0.to_string_lossy().serialize(serializer)
        }
    }
}

#[cfg(unix)]
#[derive(Deserialize)]
#[serde(untagged)]
enum PathRepresentation {
    Text(String),
    Bytes(Vec<u8>),
}

#[cfg(windows)]
#[derive(Deserialize)]
#[serde(untagged)]
enum PathRepresentation {
    Text(String),
    Bytes(Vec<u16>),
}

impl<'de> Deserialize<'de> for JournalPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            return match PathRepresentation::deserialize(deserializer)? {
                PathRepresentation::Text(text) => Ok(Self(PathBuf::from(text))),
                PathRepresentation::Bytes(bytes) => {
                    Ok(Self(PathBuf::from(std::ffi::OsString::from_vec(bytes))))
                }
            };
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            return match PathRepresentation::deserialize(deserializer)? {
                PathRepresentation::Text(text) => Ok(Self(PathBuf::from(text))),
                PathRepresentation::Bytes(wide) => {
                    Ok(Self(PathBuf::from(std::ffi::OsString::from_wide(&wide))))
                }
            };
        }
        #[cfg(not(any(unix, windows)))]
        {
            let text = String::deserialize(deserializer)?;
            Ok(Self(PathBuf::from(text)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal() -> DirectoryJournal {
        DirectoryJournal::prepared(vec![
            JournalOperation::Rename {
                from: JournalPath::from(PathBuf::from("/project/a.txt")),
                to: JournalPath::from(PathBuf::from("/project/.a.tmp")),
                stage: None,
            },
            JournalOperation::Delete {
                path: JournalPath::from(PathBuf::from("/project/b.txt")),
                staged: JournalPath::from(PathBuf::from("/project/.b.tmp")),
            },
        ])
    }

    #[test]
    fn journals_round_trip() {
        let journal = journal();
        let decoded = DirectoryJournal::from_bytes(&journal.to_bytes().unwrap()).unwrap();
        assert_eq!(decoded, journal);
        assert!(decoded.is_prepared());
        assert!(!decoded.committed(Vec::new()).is_prepared());
    }

    #[test]
    fn stages_and_deferred_sources_round_trip() {
        let journal = DirectoryJournal::prepared(vec![JournalOperation::Rename {
            from: JournalPath::from(PathBuf::from("/project/a.txt")),
            to: JournalPath::from(PathBuf::from("/mnt/b.txt")),
            stage: Some(JournalPath::from(PathBuf::from(
                "/mnt/.b.txt.kawi-copy-0-0",
            ))),
        }])
        .committed(vec![JournalPath::from(PathBuf::from(
            "/project/.a.kawi-move",
        ))]);
        let decoded = DirectoryJournal::from_bytes(&journal.to_bytes().unwrap()).unwrap();
        assert_eq!(decoded, journal);
        assert_eq!(
            decoded.deferred(),
            &[JournalPath::from(PathBuf::from("/project/.a.kawi-move"))]
        );
    }

    #[test]
    fn corrupt_and_relative_journals_are_rejected() {
        assert!(DirectoryJournal::from_bytes(b"not-json").is_err());
        let relative = DirectoryJournal::prepared(vec![JournalOperation::CreateFile {
            path: JournalPath::from(PathBuf::from("relative")),
        }]);
        assert!(DirectoryJournal::from_bytes(&relative.to_bytes().unwrap()).is_err());
        let mut unsupported = journal();
        unsupported.version = DIRECTORY_JOURNAL_VERSION + 1;
        assert!(DirectoryJournal::from_bytes(&unsupported.to_bytes().unwrap()).is_err());
        let mut deferred = journal();
        deferred.deferred = vec![JournalPath::from(PathBuf::from("relative"))];
        assert!(DirectoryJournal::from_bytes(&deferred.to_bytes().unwrap()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_round_trip() {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/bad-\xff".to_vec()));
        let journal = DirectoryJournal::prepared(vec![JournalOperation::CreateFile {
            path: JournalPath::from(path.clone()),
        }]);
        let decoded = DirectoryJournal::from_bytes(&journal.to_bytes().unwrap()).unwrap();
        let JournalOperation::CreateFile { path: decoded_path } = &decoded.operations[0] else {
            panic!("unexpected operation");
        };
        assert_eq!(decoded_path.as_path(), path);
    }
}
