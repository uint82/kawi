use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use crate::options::AutosaveMode;

use super::save::SaveAfter;
use super::App;

pub(super) struct Autosave {
    mode: AutosaveMode,
    delay: Duration,
    marks: BTreeMap<usize, Mark>,
}

struct Mark {
    revision: u64,
    changed_at: Instant,
}

impl Default for Autosave {
    fn default() -> Self {
        Self {
            mode: AutosaveMode::Off,
            delay: Duration::from_millis(1000),
            marks: BTreeMap::new(),
        }
    }
}

impl Autosave {
    pub(super) fn mode(&self) -> AutosaveMode {
        self.mode
    }

    pub(super) fn set_mode(&mut self, mode: AutosaveMode) {
        self.mode = mode;
    }

    pub(super) fn delay_millis(&self) -> u64 {
        self.delay.as_millis().try_into().unwrap_or(u64::MAX)
    }

    pub(super) fn set_delay_millis(&mut self, millis: usize) {
        self.delay = Duration::from_millis(u64::try_from(millis).unwrap_or(u64::MAX));
    }

    fn observe(&mut self, number: usize, revision: u64, dirty: bool, now: Instant) {
        if !dirty {
            self.marks.remove(&number);
            return;
        }
        match self.marks.get_mut(&number) {
            Some(mark) if mark.revision == revision => {}
            Some(mark) => {
                mark.revision = revision;
                mark.changed_at = now;
            }
            None => {
                self.marks.insert(
                    number,
                    Mark {
                        revision,
                        changed_at: now,
                    },
                );
            }
        }
    }

    fn retain(&mut self, numbers: &BTreeSet<usize>) {
        self.marks.retain(|number, _| numbers.contains(number));
    }

    fn mark_attempt(&mut self, number: usize, now: Instant) {
        if let Some(mark) = self.marks.get_mut(&number) {
            mark.changed_at = now;
        }
    }

    fn due(&self, number: usize, now: Instant) -> bool {
        self.marks
            .get(&number)
            .is_some_and(|mark| now.duration_since(mark.changed_at) >= self.delay)
    }

    fn time_until_due(&self, number: usize, now: Instant) -> Option<Duration> {
        let mark = self.marks.get(&number)?;
        Some(
            self.delay
                .saturating_sub(now.duration_since(mark.changed_at)),
        )
    }
}

impl App {
    pub(super) fn autosave_tick(&mut self) -> bool {
        self.autosave_observe();
        if !self.autosave.mode().idle() || self.pending_save.is_some() {
            return false;
        }
        let number = self.editor.buffer_number();
        let now = Instant::now();
        if !self.autosave_eligible(number) || !self.autosave.due(number, now) {
            return false;
        }
        self.autosave.mark_attempt(number, now);
        self.request_save(false, SaveAfter::None, false);
        true
    }

    pub(super) fn autosave_focus_lost(&mut self) -> bool {
        self.autosave_observe();
        if !self.autosave.mode().focus() || self.pending_save.is_some() {
            return false;
        }
        let number = self.editor.buffer_number();
        if !self.autosave_eligible(number) {
            return false;
        }
        self.autosave.mark_attempt(number, Instant::now());
        self.request_save(false, SaveAfter::None, false);
        true
    }

    pub(super) fn autosave_wait_timeout(&self) -> Option<Duration> {
        if !self.autosave.mode().idle() || self.pending_save.is_some() {
            return None;
        }
        let number = self.editor.buffer_number();
        if !self.autosave_eligible(number) {
            return None;
        }
        self.autosave.time_until_due(number, Instant::now())
    }

    fn autosave_observe(&mut self) {
        let numbers = self.editor.all_buffer_numbers();
        let current: BTreeSet<usize> = numbers.iter().copied().collect();
        self.autosave.retain(&current);
        let now = Instant::now();
        for number in numbers {
            let revision = self.editor.buffer_revision_at(number).unwrap_or_default();
            let dirty = self.editor.buffer_is_dirty(number).unwrap_or(false);
            self.autosave.observe(number, revision, dirty, now);
        }
    }

    fn autosave_eligible(&self, number: usize) -> bool {
        self.editor.buffer_path(number).is_some()
            && !self.directory.contains(number)
            && !self.editor.buffer_is_readonly(number).unwrap_or(false)
            && self.editor.buffer_is_dirty(number).unwrap_or(false)
            && !self.disk_conflict(number)
            && !self.disk_deleted(number)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crossterm::event::Event;
    use kawi_buffer::Buffer;
    use kawi_editor::Action;

    use super::*;
    use crate::keymap::Flavor;

    fn app_with(name: &str) -> (std::path::PathBuf, std::path::PathBuf, App) {
        let dir = crate::test_support::temp_dir(name);
        let path = dir.join("a.txt");
        fs::write(&path, "alpha\n").expect("write");
        let mut app = App::new(
            Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        );
        app.sync_watched_files();
        (dir, path, app)
    }

    fn edit(app: &mut App, text: &str) {
        app.editor.apply(Action::EnterInsert);
        for ch in text.chars() {
            app.editor.apply(Action::InsertChar(ch));
        }
        app.editor.apply(Action::ExitInsert);
    }

    #[test]
    fn autosave_is_off_by_default() {
        let (dir, path, mut app) = app_with("default");
        edit(&mut app, "x");
        assert!(!app.autosave_tick());
        assert_eq!(fs::read_to_string(path).expect("disk"), "alpha\n");
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn idle_autosave_waits_for_the_delay() {
        let (dir, path, mut app) = app_with("idle");
        app.autosave.set_mode(AutosaveMode::Idle);
        app.autosave.set_delay_millis(60_000);
        edit(&mut app, "x");
        assert!(!app.autosave_tick());
        assert_eq!(fs::read_to_string(&path).expect("disk"), "alpha\n");
        app.autosave.set_delay_millis(0);
        assert!(app.autosave_tick());
        assert_eq!(fs::read_to_string(path).expect("disk"), "xalpha\n");
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn idle_autosave_preserves_a_wait_timeout() {
        let (dir, _path, mut app) = app_with("timeout");
        app.autosave.set_mode(AutosaveMode::Idle);
        edit(&mut app, "x");
        app.autosave_tick();
        assert!(app.autosave_wait_timeout().is_some());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn focus_autosave_saves_only_in_focus_modes() {
        let (dir, path, mut app) = app_with("focus");
        app.autosave.set_mode(AutosaveMode::Focus);
        edit(&mut app, "x");
        assert!(app.autosave_focus_lost());
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        edit(&mut app, "y");
        app.autosave.set_mode(AutosaveMode::Idle);
        assert!(!app.autosave_focus_lost());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn focus_lost_events_route_to_autosave() {
        let (dir, path, mut app) = app_with("focus_event");
        app.autosave.set_mode(AutosaveMode::Focus);
        edit(&mut app, "x");
        app.update(Event::FocusLost).expect("focus event");
        assert_eq!(fs::read_to_string(path).expect("disk"), "xalpha\n");
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn conflicts_block_autosave() {
        let (dir, path, mut app) = app_with("conflict");
        app.autosave.set_mode(AutosaveMode::All);
        app.autosave.set_delay_millis(0);
        edit(&mut app, "x");
        fs::write(&path, "theirs\n").expect("external");
        app.handle_disk_paths(std::slice::from_ref(&path));
        assert!(!app.autosave_tick());
        assert!(!app.autosave_focus_lost());
        assert_eq!(fs::read_to_string(path).expect("disk"), "theirs\n");
        assert!(app.editor.is_dirty());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn deleted_files_block_autosave() {
        let (dir, path, mut app) = app_with("deleted");
        app.autosave.set_mode(AutosaveMode::All);
        app.autosave.set_delay_millis(0);
        edit(&mut app, "x");
        fs::remove_file(&path).expect("remove");
        app.handle_disk_paths(std::slice::from_ref(&path));
        assert!(!app.autosave_tick());
        assert!(!app.autosave_focus_lost());
        fs::remove_dir_all(dir).expect("cleanup");
    }
}
