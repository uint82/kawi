use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kawi_buffer::UndoStore;
use sha2::{Digest, Sha256};

use super::path_match::glob_matches;
use super::App;

const DEFAULT_EXCLUSIONS: &[&str] = &["/tmp/*", "*.gpg", "*/.ssh/*"];
const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_AGE: Duration = Duration::from_secs(90 * 24 * 60 * 60);

pub(super) struct UndoState {
    store: Option<UndoStore>,
    enabled: bool,
    exclusions: Vec<String>,
    attached: BTreeMap<usize, String>,
    persisted: BTreeMap<usize, PersistedUndo>,
}

struct PersistedUndo {
    path: String,
    revision: u64,
    source_hash: Option<[u8; 32]>,
}

impl Default for UndoState {
    fn default() -> Self {
        Self {
            store: None,
            enabled: true,
            exclusions: DEFAULT_EXCLUSIONS
                .iter()
                .map(|pattern| (*pattern).to_string())
                .collect(),
            attached: BTreeMap::new(),
            persisted: BTreeMap::new(),
        }
    }
}

impl UndoState {
    pub(super) fn configure(&mut self, dir: PathBuf) {
        self.store = Some(UndoStore::new(dir));
        self.attached.clear();
        self.persisted.clear();
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.attached.clear();
            self.persisted.clear();
        }
        self.enabled = enabled;
    }

    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(super) fn set_exclusions(&mut self, exclusions: Vec<String>) {
        self.exclusions = exclusions;
        self.attached.clear();
        self.persisted.clear();
    }

    #[cfg(test)]
    pub(super) fn exclusions(&self) -> &[String] {
        &self.exclusions
    }

    fn attached_path(&self, number: usize) -> Option<&str> {
        self.attached.get(&number).map(String::as_str)
    }

    fn attach(&mut self, number: usize, path: String) {
        self.attached.insert(number, path);
    }

    fn detach(&mut self, number: usize) {
        self.attached.remove(&number);
        self.persisted.remove(&number);
    }

    fn retain(&mut self, numbers: &[usize]) {
        self.attached.retain(|number, _| numbers.contains(number));
        self.persisted.retain(|number, _| numbers.contains(number));
    }

    fn is_persisted(
        &self,
        number: usize,
        path: &str,
        revision: u64,
        source_hash: &[u8; 32],
    ) -> bool {
        self.persisted.get(&number).is_some_and(|persisted| {
            persisted.path == path
                && persisted.revision == revision
                && persisted
                    .source_hash
                    .as_ref()
                    .is_none_or(|stored| stored == source_hash)
        })
    }

    fn remember_persisted(
        &mut self,
        number: usize,
        path: String,
        revision: u64,
        source_hash: Option<[u8; 32]>,
    ) {
        self.persisted.insert(
            number,
            PersistedUndo {
                path,
                revision,
                source_hash,
            },
        );
    }

    fn should_persist(&self, path: &Path) -> bool {
        self.enabled
            && self.store.is_some()
            && !self
                .exclusions
                .iter()
                .any(|pattern| glob_matches(pattern, &canonical_display(path)))
    }

    pub(super) fn move_prefix(&mut self, old: &Path, new: &Path) {
        if let Some(store) = self.store.as_ref() {
            if let Err(error) = store.move_prefix(old, new) {
                log_undo(old, error);
            }
        }
        self.attached.clear();
        self.persisted.clear();
    }

    pub(super) fn remove_prefix(&mut self, path: &Path) {
        if let Some(store) = self.store.as_ref() {
            if let Err(error) = store.remove_prefix(path) {
                log_undo(path, error);
            }
        }
        self.attached.clear();
        self.persisted.clear();
    }

    pub(super) fn spawn_prune(&self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        if !self.enabled {
            return;
        }
        std::thread::spawn(move || {
            if let Err(error) = store.prune(MAX_TOTAL_BYTES, MAX_AGE) {
                eprintln!("kawi: undo prune: {error}");
            }
        });
    }
}

impl App {
    pub(super) fn sync_undo_histories(&mut self) {
        if !self.undo.enabled {
            return;
        }
        let Some(store) = self.undo.store.clone() else {
            return;
        };
        let numbers = self.editor.buffer_numbers();
        self.undo.retain(&numbers);
        for number in numbers {
            let Some(path) = self.editor.buffer_path(number).map(str::to_string) else {
                self.undo.detach(number);
                continue;
            };
            if self.undo.attached_path(number) == Some(path.as_str()) {
                continue;
            }
            if self.skip_undo_buffer(number, &path) {
                self.undo.attach(number, path);
                continue;
            }
            let Some(hash) = self.editor.buffer_source_hash(number) else {
                self.undo.attach(number, path);
                continue;
            };
            match store.load(Path::new(&path), &hash) {
                Ok(Some(history)) => {
                    if self.editor.restore_buffer_history(number, &history) {
                        if let Some(revision) = self.editor.buffer_revision_at(number) {
                            self.undo.remember_persisted(
                                number,
                                path.clone(),
                                revision,
                                Some(hash),
                            );
                        }
                    } else {
                        log_undo(Path::new(&path), "invalid history");
                        store.remove(Path::new(&path));
                    }
                }
                Ok(None) => {}
                Err(error) => log_undo(Path::new(&path), error),
            }
            self.undo.attach(number, path);
        }
    }

    pub(super) fn persist_undo(&mut self, number: usize) {
        if !self.undo.enabled {
            return;
        }
        let Some(store) = self.undo.store.clone() else {
            return;
        };
        let Some(path) = self.editor.buffer_path(number).map(str::to_string) else {
            return;
        };
        if !Path::new(&path).exists() {
            store.remove(Path::new(&path));
            return;
        }
        if self.skip_undo_buffer(number, &path) {
            return;
        }
        let Some(revision) = self.editor.buffer_revision_at(number) else {
            return;
        };
        let Some(hash) = self.editor.buffer_source_hash(number) else {
            return;
        };
        if self.undo.is_persisted(number, &path, revision, &hash) {
            return;
        }
        let Some(history) = self.editor.buffer_history(number) else {
            return;
        };
        if !history.has_changes() {
            store.remove(Path::new(&path));
            self.undo
                .remember_persisted(number, path.clone(), revision, None);
            self.undo.attach(number, path);
            return;
        }
        if !disk_matches(Path::new(&path), &hash) {
            return;
        }
        if let Err(error) = store.save(Path::new(&path), &hash, &history) {
            log_undo(Path::new(&path), error);
        } else {
            self.undo
                .remember_persisted(number, path.clone(), revision, Some(hash));
        }
        self.undo.attach(number, path);
    }

    pub(super) fn persist_clean_undos(&mut self) {
        for number in self.editor.buffer_numbers() {
            self.persist_undo(number);
        }
    }

    fn skip_undo_buffer(&self, number: usize, path: &str) -> bool {
        if self.directory.contains(number)
            || self.editor.buffer_is_readonly(number).unwrap_or(true)
            || self.editor.buffer_is_dirty(number).unwrap_or(true)
            || self.git_commit.editor_buffer() == Some(number)
            || is_commit_message_path(Path::new(path))
            || !self.undo.should_persist(Path::new(path))
        {
            return true;
        }
        source_is_too_large(Path::new(path))
    }
}

fn is_commit_message_path(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == "COMMIT_EDITMSG")
}

fn source_is_too_large(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_SOURCE_BYTES)
}

fn disk_matches(path: &Path, hash: &[u8; 32]) -> bool {
    std::fs::read(path).is_ok_and(|bytes| Sha256::digest(&bytes).as_slice() == hash)
}

fn canonical_display(path: &Path) -> String {
    let path = std::fs::canonicalize(path)
        .unwrap_or_else(|_| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()));
    path.to_string_lossy().into_owned()
}

fn log_undo(path: &Path, message: impl std::fmt::Display) {
    eprintln!("kawi: undo {}: {message}", path.display());
}

#[cfg(test)]
mod tests;
