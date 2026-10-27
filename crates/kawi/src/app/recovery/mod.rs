use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;
use kawi_editor::{paths, ViewState};

use super::App;
use crate::recovery::{
    clear_panic_snapshots, register_panic_snapshots, update_panic_snapshots, BufferObservation,
    Fingerprint, PanicSnapshot, PendingBuffer, RecoveryManager, RecoveryStart, Snapshot,
    SnapshotHeader, SNAPSHOT_VERSION,
};
use crate::session::SessionView;
pub(super) use prompt::RecoveryPrompt;
use prompt::{recovery_choice, RecoveryCandidate, RecoveryChoice};

mod prompt;
#[cfg(test)]
mod tests;

impl App {
    pub(crate) fn start_recovery(
        &mut self,
        root: &Path,
        cwd: &Path,
        restrict_to_open: bool,
    ) -> Option<String> {
        match RecoveryManager::start(root, cwd, self.events.wake()) {
            RecoveryStart::Disabled(warning) => Some(warning),
            RecoveryStart::Active {
                manager,
                snapshots,
                warning,
            } => {
                let candidates =
                    self.recovery_candidates(snapshots, restrict_to_open, manager.cwd());
                register_panic_snapshots(manager.store_clone());
                self.recovery = Some(*manager);
                self.recovery_prompt = RecoveryPrompt::new(candidates);
                if warning.is_none() {
                    self.show_recovery_prompt_if_ready();
                }
                warning
            }
        }
    }

    pub(super) fn show_recovery_prompt_if_ready(&mut self) {
        let Some(prompt) = self.recovery_prompt.as_ref() else {
            return;
        };
        if prompt.shown() || self.pager.is_some() {
            return;
        }
        let Some(lines) = prompt.lines(self.screen_width) else {
            self.recovery_prompt = None;
            return;
        };
        if let Some(prompt) = self.recovery_prompt.as_mut() {
            prompt.mark_shown();
        }
        self.show_pager(lines);
    }

    pub(super) fn recovery_prompt_visible(&self) -> bool {
        self.recovery_prompt
            .as_ref()
            .is_some_and(RecoveryPrompt::shown)
    }

    pub(super) fn handle_recovery_prompt_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let Some(choice) = recovery_choice(key) else {
            return Ok(false);
        };
        let Some(mut prompt) = self.recovery_prompt.take() else {
            return Ok(false);
        };
        let Some(candidate) = prompt.pop() else {
            self.pager = None;
            return Ok(false);
        };
        self.pager = None;
        match choice {
            RecoveryChoice::Recover => {
                if let Some(error) = self.recover_snapshot(&candidate.snapshot) {
                    self.show_message(error);
                }
            }
            RecoveryChoice::Discard => {
                if let Some(error) = self.discard_recovery_snapshot(&candidate.snapshot) {
                    self.show_message(error);
                }
            }
            RecoveryChoice::Skip => {}
        }
        if prompt.is_empty() {
            self.show_message("Recovery finished".to_string());
        } else {
            self.recovery_prompt = Some(prompt);
            self.show_recovery_prompt_if_ready();
        }
        Ok(false)
    }

    pub(super) fn recovery_tick(&mut self) -> bool {
        let Some(manager) = self.recovery.as_ref() else {
            return false;
        };
        let cwd = manager.cwd().to_path_buf();
        let observations = self.recovery_observations(&cwd);
        let Some(mut manager) = self.recovery.take() else {
            return false;
        };
        let store = manager.store_clone();
        let mut errors = manager.drain_errors();
        let now = Instant::now();
        manager.observe(&observations, now);
        if manager.due(now) {
            let pending = manager.pending_buffers();
            let snapshots = self.recovery_snapshots(&pending);
            if !manager.queue(snapshots) {
                errors.push("recovery: snapshot writer stopped".to_string());
            }
        }
        update_panic_snapshots(&store, self.panic_snapshots(&observations));
        self.recovery = Some(manager);
        if errors.is_empty() {
            return false;
        }
        self.show_message(errors.join("\n"));
        true
    }

    pub(super) fn recovery_wait_timeout(&self) -> Option<Duration> {
        self.recovery
            .as_ref()
            .and_then(|manager| manager.deadline(Instant::now()))
    }

    pub(super) fn finish_recovery_normal(&mut self) {
        let Some(mut manager) = self.recovery.take() else {
            return;
        };
        let store = manager.store_clone();
        let cwd = manager.cwd().to_path_buf();
        let observations = self.recovery_observations(&cwd);
        manager.finish_clean(&observations);
        clear_panic_snapshots(&store);
    }

    pub(super) fn finish_recovery_abnormal(&mut self) {
        let Some(mut manager) = self.recovery.take() else {
            return;
        };
        let store = manager.store_clone();
        let cwd = manager.cwd().to_path_buf();
        let observations = self.recovery_observations(&cwd);
        let snapshots = self.recovery_snapshots_for(&observations);
        let _ = manager.finish_abnormal(&observations, snapshots);
        clear_panic_snapshots(&store);
    }

    pub(super) fn discard_recovery_buffers(&mut self) {
        let Some(mut manager) = self.recovery.take() else {
            self.recovery_prompt = None;
            return;
        };
        let store = manager.store_clone();
        let cwd = manager.cwd().to_path_buf();
        let mut ids: BTreeSet<String> = self
            .recovery_observations(&cwd)
            .into_iter()
            .map(|buffer| buffer.id)
            .collect();
        ids.extend(manager.tracked_ids());
        let ids: Vec<String> = ids.into_iter().collect();
        let _ = manager.discard_buffers(&ids);
        clear_panic_snapshots(&store);
        self.recovery_prompt = None;
        self.pager = None;
    }

    pub(super) fn clear_recovery_buffer(&mut self, number: usize) {
        let Some(id) = self.editor.buffer_id(number).map(str::to_string) else {
            return;
        };
        if let Some(manager) = self.recovery.as_mut() {
            let _ = manager.remove_snapshot(&id);
        }
    }

    fn recovery_candidates(
        &self,
        snapshots: Vec<Snapshot>,
        restrict_to_open: bool,
        cwd: &Path,
    ) -> Vec<RecoveryCandidate> {
        let open_ids: BTreeSet<&str> = self
            .editor
            .all_buffer_numbers()
            .into_iter()
            .filter_map(|number| self.editor.buffer_id(number))
            .collect();
        let open_paths: BTreeSet<String> = self
            .editor
            .all_buffer_numbers()
            .into_iter()
            .filter_map(|number| self.editor.buffer_path(number))
            .map(|path| normalized_path(path, cwd))
            .collect();
        snapshots
            .into_iter()
            .filter(|snapshot| {
                !restrict_to_open
                    || open_ids.contains(snapshot.header.buffer_id.as_str())
                    || snapshot
                        .header
                        .path
                        .as_deref()
                        .is_some_and(|path| open_paths.contains(&normalized_path(path, cwd)))
            })
            .map(|snapshot| RecoveryCandidate {
                conflict: snapshot_conflicts(&snapshot),
                snapshot,
            })
            .collect()
    }

    fn recover_snapshot(&mut self, snapshot: &Snapshot) -> Option<String> {
        let Some(number) = self.recovery_buffer_number(snapshot) else {
            return Some(format!(
                "recovery: could not restore {}",
                snapshot.header.name
            ));
        };
        if self.editor.buffer_id(number) != Some(snapshot.header.buffer_id.as_str()) {
            self.editor
                .set_buffer_id(number, snapshot.header.buffer_id.clone());
        }
        if !self.editor.restore_buffer_text(number, &snapshot.text) {
            return Some(format!(
                "recovery: could not restore {}",
                snapshot.header.name
            ));
        }
        let view = self
            .editor
            .clamp_view_state(number, ViewState::from(snapshot.header.view));
        self.apply_recovered_view(number, view);
        if let Some(manager) = self.recovery.as_mut() {
            manager.track_recovered(
                snapshot.header.buffer_id.clone(),
                number,
                snapshot.header.name.clone(),
                snapshot.header.path.clone(),
                self.editor.buffer_revision_at(number).unwrap_or(0),
            );
        }
        None
    }

    fn recovery_buffer_number(&mut self, snapshot: &Snapshot) -> Option<usize> {
        if let Some(number) = self.editor.find_buffer_by_id(&snapshot.header.buffer_id) {
            return Some(number);
        }
        let cwd = self
            .recovery
            .as_ref()
            .map(|manager| manager.cwd().to_path_buf())?;
        match snapshot.header.path.as_deref() {
            Some(path) => {
                let path = normalized_path(path, &cwd);
                if let Some(number) = self.find_buffer_by_path(&path, &cwd) {
                    return Some(number);
                }
                self.sync_active_window();
                self.editor.open_file_buffer(&path).ok()?;
                Some(self.editor.buffer_number())
            }
            None => {
                self.sync_active_window();
                self.editor.open_empty_buffer();
                Some(self.editor.buffer_number())
            }
        }
    }

    fn find_buffer_by_path(&self, path: &str, cwd: &Path) -> Option<usize> {
        self.editor.all_buffer_numbers().into_iter().find(|number| {
            self.editor
                .buffer_path(*number)
                .is_some_and(|buffer_path| normalized_path(buffer_path, cwd) == path)
        })
    }

    fn apply_recovered_view(&mut self, number: usize, view: ViewState) {
        let updated = self.windows.set_view_for_buffer(number, view);
        if updated == 0 && self.editor.buffer_number() == number {
            let active = self.windows.active_view_mut();
            active.buffer = number;
            active.view = view;
        }
        if self.editor.buffer_number() == number {
            self.editor.restore_view_state(view);
            self.reveal_cursor();
        }
        self.sync_active_window();
    }

    fn discard_recovery_snapshot(&mut self, snapshot: &Snapshot) -> Option<String> {
        self.recovery
            .as_mut()?
            .remove_snapshot(&snapshot.header.buffer_id)
    }

    fn recovery_observations(&self, cwd: &Path) -> Vec<BufferObservation> {
        self.editor
            .all_buffer_numbers()
            .into_iter()
            .filter(|number| !self.directory.contains(*number))
            .filter_map(|number| {
                Some(BufferObservation {
                    number,
                    id: self.editor.buffer_id(number)?.to_string(),
                    name: self
                        .editor
                        .buffer_name(number)
                        .unwrap_or("[No Name]")
                        .to_string(),
                    path: self
                        .editor
                        .buffer_path(number)
                        .map(|path| absolute_path(path, cwd)),
                    revision: self.editor.buffer_revision_at(number)?,
                    dirty: self.editor.buffer_is_dirty(number)?,
                })
            })
            .collect()
    }

    fn recovery_snapshots(&self, pending: &[PendingBuffer]) -> Vec<Snapshot> {
        pending
            .iter()
            .filter_map(|buffer| {
                let text = self.editor.buffer_text(buffer.number)?;
                Some(self.recovery_snapshot(
                    buffer.number,
                    &buffer.id,
                    &buffer.name,
                    buffer.path.as_deref(),
                    buffer.revision,
                    text,
                ))
            })
            .collect()
    }

    fn recovery_snapshots_for(&self, observations: &[BufferObservation]) -> Vec<Snapshot> {
        observations
            .iter()
            .filter(|buffer| buffer.dirty)
            .filter_map(|buffer| {
                let text = self.editor.buffer_text(buffer.number)?;
                Some(self.recovery_snapshot(
                    buffer.number,
                    &buffer.id,
                    &buffer.name,
                    buffer.path.as_deref(),
                    buffer.revision,
                    text,
                ))
            })
            .collect()
    }

    fn recovery_snapshot(
        &self,
        number: usize,
        id: &str,
        name: &str,
        path: Option<&str>,
        revision: u64,
        text: String,
    ) -> Snapshot {
        Snapshot {
            header: self.recovery_header(number, id, name, path, revision),
            text,
        }
    }

    fn recovery_header(
        &self,
        number: usize,
        id: &str,
        name: &str,
        path: Option<&str>,
        revision: u64,
    ) -> SnapshotHeader {
        let view = self
            .windows
            .view_for_buffer(number)
            .unwrap_or_else(|| self.editor.view_state());
        let view = self.editor.clamp_view_state(number, view);
        SnapshotHeader {
            version: SNAPSHOT_VERSION,
            buffer_id: id.to_string(),
            name: name.to_string(),
            path: path.map(str::to_string),
            revision,
            fingerprint: None,
            view: SessionView::from(view),
        }
    }

    fn panic_snapshots(&self, observations: &[BufferObservation]) -> Vec<PanicSnapshot> {
        observations
            .iter()
            .filter(|buffer| buffer.dirty)
            .filter_map(|buffer| {
                Some(PanicSnapshot {
                    header: self.recovery_header(
                        buffer.number,
                        &buffer.id,
                        &buffer.name,
                        buffer.path.as_deref(),
                        buffer.revision,
                    ),
                    text: self.editor.buffer_rope(buffer.number)?,
                })
            })
            .collect()
    }
}

fn snapshot_conflicts(snapshot: &Snapshot) -> bool {
    let Some(path) = snapshot.header.path.as_deref() else {
        return false;
    };
    match (
        snapshot.header.fingerprint.as_ref(),
        Fingerprint::capture(Path::new(path)),
    ) {
        (Some(saved), Some(current)) => !saved.matches(&current),
        (None, None) => false,
        _ => true,
    }
}

fn absolute_path(path: &str, cwd: &Path) -> String {
    paths::absolute(path, cwd).to_string_lossy().into_owned()
}

fn normalized_path(path: &str, cwd: &Path) -> String {
    let absolute = paths::absolute(path, cwd);
    std::fs::canonicalize(&absolute)
        .unwrap_or(absolute)
        .to_string_lossy()
        .into_owned()
}
