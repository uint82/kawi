use std::path::PathBuf;

use kawi_watch::Fingerprint;

use crate::app::identity::{file_identity, FileIdentity};

use super::{Observation, ObservedFile};

pub(super) struct DiskFile {
    pub(super) path: PathBuf,
    pub(super) identity: Option<FileIdentity>,
    pub(super) base: Option<Fingerprint>,
    pub(super) current: Option<Fingerprint>,
    pub(super) changed: bool,
    pub(super) deleted: bool,
    pub(super) conflict: bool,
    pub(super) dismissed: bool,
}

impl DiskFile {
    pub(super) fn new(path: PathBuf) -> Self {
        let observed = ObservedFile::read(&path);
        Self::from_observed(path, observed)
    }

    pub(super) fn from_fingerprint(path: PathBuf, fingerprint: Option<Fingerprint>) -> Self {
        Self::from_observed(
            path.clone(),
            ObservedFile {
                fingerprint,
                identity: file_identity(&path),
            },
        )
    }

    fn from_observed(path: PathBuf, observed: ObservedFile) -> Self {
        Self {
            path,
            identity: observed.identity,
            base: observed.fingerprint,
            current: observed.fingerprint,
            changed: false,
            deleted: false,
            conflict: false,
            dismissed: false,
        }
    }

    pub(super) fn observe(&mut self, observed: ObservedFile) -> Observation {
        if observed.fingerprint.is_some() {
            self.identity = observed.identity;
        }
        let Some(observed) = observed.fingerprint else {
            if self.base.is_none() && self.current.is_none() {
                return Observation::Ignored;
            }
            if self.deleted {
                return Observation::Ignored;
            }
            self.current = None;
            self.changed = true;
            self.deleted = true;
            self.conflict = false;
            self.dismissed = false;
            return Observation::Deleted;
        };
        if self
            .base
            .as_ref()
            .is_some_and(|base| base.matches(&observed))
        {
            self.accept(observed);
            return Observation::Same;
        }
        if !self.deleted
            && self
                .current
                .as_ref()
                .is_some_and(|current| current.matches(&observed))
        {
            return Observation::Ignored;
        }
        self.current = Some(observed);
        self.deleted = false;
        Observation::Changed
    }

    pub(super) fn accept(&mut self, fingerprint: Fingerprint) {
        self.identity = file_identity(&self.path);
        self.base = Some(fingerprint);
        self.current = Some(fingerprint);
        self.changed = false;
        self.deleted = false;
        self.conflict = false;
        self.dismissed = false;
    }

    pub(super) fn accept_current(&mut self) {
        self.base = self.current;
        self.changed = false;
        self.deleted = false;
        self.conflict = false;
        self.dismissed = false;
    }

    pub(super) fn mark_changed(&mut self) {
        self.changed = true;
        self.conflict = false;
        self.dismissed = false;
    }

    pub(super) fn mark_conflict(&mut self) {
        self.changed = true;
        self.conflict = true;
        self.dismissed = false;
    }

    pub(super) fn record_saved(&mut self, fingerprint: Option<Fingerprint>) {
        self.identity = file_identity(&self.path);
        self.base = fingerprint;
        self.current = fingerprint;
        self.changed = false;
        self.deleted = false;
        self.conflict = false;
        self.dismissed = false;
    }
}
