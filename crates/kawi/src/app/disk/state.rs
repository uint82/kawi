use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kawi_editor::paths;
use kawi_watch::Fingerprint;

use crate::app::identity::{file_identity, find_moved_entry, FileIdentity};

mod file;
use file::DiskFile;

pub(super) const STATUS_CHANGED: &str = "changed";
pub(super) const STATUS_DELETED: &str = "deleted";

#[derive(Default)]
pub(in crate::app) struct DiskState {
    files: BTreeMap<usize, DiskFile>,
    pub(super) watched: BTreeSet<PathBuf>,
}

#[derive(Clone, Copy)]
pub(super) struct ObservedFile {
    pub(super) fingerprint: Option<Fingerprint>,
    pub(super) identity: Option<FileIdentity>,
}

impl ObservedFile {
    pub(super) fn read(path: &Path, previous: Option<Fingerprint>) -> Self {
        Self {
            fingerprint: Fingerprint::refresh(path, previous.as_ref()).ok(),
            identity: file_identity(path),
        }
    }

    pub(super) fn is_missing(&self) -> bool {
        self.fingerprint.is_none() && self.identity.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Observation {
    Ignored,
    Same,
    Changed,
    Deleted,
}

impl DiskState {
    pub(super) fn sync(&mut self, files: &[(usize, PathBuf)]) -> Option<BTreeSet<PathBuf>> {
        let numbers: BTreeSet<usize> = files.iter().map(|(number, _)| *number).collect();
        self.files.retain(|number, _| numbers.contains(number));
        for (number, path) in files {
            match self.files.get_mut(number) {
                Some(file) if file.path == *path => {}
                Some(file) => *file = DiskFile::new(path.clone()),
                None => {
                    self.files.insert(*number, DiskFile::new(path.clone()));
                }
            }
        }
        let parents: BTreeSet<PathBuf> = files
            .iter()
            .filter_map(|(_, path)| path.parent().map(Path::to_path_buf))
            .collect();
        if parents == self.watched {
            return None;
        }
        self.watched = parents.clone();
        Some(parents)
    }

    pub(in crate::app) fn path(&self, number: usize) -> Option<&Path> {
        self.files.get(&number).map(|file| file.path.as_path())
    }

    pub(super) fn fingerprint(&self, number: usize) -> Option<Fingerprint> {
        self.files.get(&number).and_then(DiskFile::fingerprint)
    }

    pub(super) fn status(&self, number: usize) -> Option<&'static str> {
        let file = self.files.get(&number)?;
        if file.deleted {
            Some(STATUS_DELETED)
        } else if file.changed {
            Some(STATUS_CHANGED)
        } else {
            None
        }
    }

    pub(super) fn conflict(&self, number: usize) -> bool {
        self.files.get(&number).is_some_and(|file| file.conflict)
    }

    pub(super) fn any_conflict(&self) -> bool {
        self.files.values().any(|file| file.conflict)
    }

    pub(super) fn needs_prompt(&self, number: usize) -> bool {
        self.files
            .get(&number)
            .is_some_and(|file| file.conflict && !file.dismissed)
    }

    pub(super) fn open_prompt(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.dismissed = false;
        }
    }

    pub(super) fn dismiss(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.dismissed = true;
        }
    }

    pub(super) fn matching_buffers(&self, paths: &[PathBuf]) -> Vec<usize> {
        self.files
            .iter()
            .filter(|(_, file)| paths.iter().any(|path| paths::same_file(path, &file.path)))
            .map(|(number, _)| *number)
            .collect()
    }

    pub(super) fn moved_path(&self, number: usize, search: &BTreeSet<PathBuf>) -> Option<PathBuf> {
        let file = self.files.get(&number)?;
        let identity = file.identity?;
        find_moved_entry(identity, search, &file.path, file_identity)
    }

    pub(super) fn retarget(&mut self, number: usize, path: PathBuf) {
        if let Some(file) = self.files.get_mut(&number) {
            file.path = path;
            file.identity = None;
        }
    }

    pub(super) fn observe(&mut self, number: usize, observed: ObservedFile) -> Observation {
        let Some(file) = self.files.get_mut(&number) else {
            return Observation::Ignored;
        };
        file.observe(observed)
    }

    pub(super) fn accept(&mut self, number: usize, fingerprint: Fingerprint) {
        if let Some(file) = self.files.get_mut(&number) {
            file.accept(fingerprint);
        }
    }

    pub(super) fn accept_current(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.accept_current();
        }
    }

    pub(super) fn mark_changed(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.mark_changed();
        }
    }

    pub(super) fn mark_conflict(&mut self, number: usize) {
        if let Some(file) = self.files.get_mut(&number) {
            file.mark_conflict();
        }
    }

    pub(super) fn record_saved(
        &mut self,
        number: usize,
        path: PathBuf,
        fingerprint: Option<Fingerprint>,
    ) {
        match self.files.get_mut(&number) {
            Some(file) if file.path == path => file.record_saved(fingerprint),
            Some(file) => {
                *file = DiskFile::from_fingerprint(path, fingerprint);
            }
            None => {
                self.files
                    .insert(number, DiskFile::from_fingerprint(path, fingerprint));
            }
        }
    }
}
