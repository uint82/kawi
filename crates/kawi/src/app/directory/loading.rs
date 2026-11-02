use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use super::diff::Change;
use super::loader::{DirectoryLoader, LoadResult};
use super::metadata::MetadataColumns;
use super::sort::SortMode;
use crate::app::save::SaveAfter;

#[derive(Default)]
pub(in crate::app) struct DirectoryLoading {
    wake: Option<Sender<()>>,
    loader: Option<DirectoryLoader>,
    loads: BTreeMap<u64, PendingLoad>,
    focus_after_load: BTreeMap<usize, String>,
}

pub(super) struct PendingLoad {
    pub(super) number: usize,
    pub(super) root: PathBuf,
    pub(super) purpose: LoadPurpose,
}

pub(super) enum LoadPurpose {
    Display {
        initial: bool,
    },
    Save {
        changes: Vec<Change>,
        force: bool,
        after: SaveAfter,
        visible: bool,
    },
    SaveAll {
        batch: u64,
    },
}

impl LoadPurpose {
    fn needs_text(&self) -> bool {
        matches!(self, Self::Display { .. })
    }

    pub(super) fn needs_metadata(&self) -> bool {
        matches!(self, Self::Display { .. })
    }
}

pub(super) struct LoadOutcome {
    pub(super) pending: PendingLoad,
    pub(super) listing: Result<super::loader::LoadedListing, String>,
}

impl DirectoryLoading {
    pub(in crate::app) fn set_wake(&mut self, wake: Sender<()>) {
        self.wake = Some(wake);
    }

    pub(in crate::app) fn is_loading(&self, number: usize) -> bool {
        self.loads.values().any(|load| load.number == number)
    }

    pub(in crate::app) fn forget(&mut self, number: usize) {
        self.loads.retain(|_, load| load.number != number);
        self.focus_after_load.remove(&number);
    }

    pub(in crate::app) fn retain_listed(&mut self, listed: &[usize]) -> Vec<usize> {
        let cancelled: Vec<usize> = self
            .loads
            .values()
            .map(|load| load.number)
            .filter(|number| !listed.contains(number))
            .collect();
        self.loads.retain(|_, load| listed.contains(&load.number));
        self.focus_after_load
            .retain(|number, _| listed.contains(number));
        cancelled
    }

    pub(in crate::app) fn defer_focus(&mut self, number: usize, text: String) {
        self.focus_after_load.insert(number, text);
    }

    pub(super) fn take_focus(&mut self, number: usize) -> Option<String> {
        self.focus_after_load.remove(&number)
    }

    pub(super) fn clear_focus(&mut self, number: usize) {
        self.focus_after_load.remove(&number);
    }

    #[cfg(test)]
    pub(in crate::app) fn has_loads(&self) -> bool {
        !self.loads.is_empty()
    }

    pub(super) fn submit(
        &mut self,
        number: usize,
        root: PathBuf,
        show_hidden: bool,
        sort: SortMode,
        columns: MetadataColumns,
        purpose: LoadPurpose,
    ) -> Result<(), String> {
        let id = self.request_load(
            root.clone(),
            show_hidden,
            sort,
            columns,
            purpose.needs_text(),
        )?;
        self.track_load(number, root, purpose, id);
        Ok(())
    }

    fn request_load(
        &mut self,
        root: PathBuf,
        show_hidden: bool,
        sort: SortMode,
        columns: MetadataColumns,
        include_text: bool,
    ) -> Result<u64, String> {
        let loader = self.ensure_loader()?;
        match loader.request(root, show_hidden, sort, columns, include_text) {
            Ok(id) => Ok(id),
            Err(error) => {
                self.loader = None;
                Err(error)
            }
        }
    }

    fn ensure_loader(&mut self) -> Result<&mut DirectoryLoader, String> {
        if self.loader.is_none() {
            let wake = self
                .wake
                .clone()
                .ok_or_else(|| "directory loader is not initialized".to_string())?;
            self.loader = Some(DirectoryLoader::new(wake)?);
        }
        self.loader
            .as_mut()
            .ok_or_else(|| "directory loader is not initialized".to_string())
    }

    fn track_load(&mut self, number: usize, root: PathBuf, purpose: LoadPurpose, id: u64) {
        self.loads.retain(|_, load| load.number != number);
        self.loads.insert(
            id,
            PendingLoad {
                number,
                root,
                purpose,
            },
        );
    }

    pub(super) fn drain(&mut self) -> Vec<LoadOutcome> {
        let mut outcomes = Vec::new();
        while let Some(result) = self.take_result() {
            let Some(pending) = self.loads.remove(&result.id) else {
                continue;
            };
            outcomes.push(LoadOutcome {
                pending,
                listing: result.listing,
            });
        }
        outcomes
    }

    fn take_result(&self) -> Option<LoadResult> {
        self.loader.as_ref().and_then(DirectoryLoader::try_recv)
    }
}
