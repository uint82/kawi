use std::time::{Duration, Instant};

use kawi_editor::Mode;
use kawi_tui::frame::KeyRow;

use super::overlay::keymenu::guide_key_row;
use super::App;
use crate::keymap::{KeyChord, KeyCodeSpec, KeyContext, Keymap};
use crate::leader::{self, LeaderPath};
use crate::vim::guide::{self, Guide};

pub(super) struct KeyGuideCache {
    pub(super) prefix: String,
    pub(super) mode: Mode,
    pub(super) breadcrumb: String,
    pub(super) hint: Option<&'static str>,
    pub(super) rows: Vec<KeyRow<'static>>,
}

impl KeyGuideCache {
    fn new(prefix: &str, mode: Mode, guide: Guide) -> Self {
        Self {
            prefix: prefix.to_string(),
            mode,
            breadcrumb: guide::guide_breadcrumb(prefix),
            hint: guide.hint,
            rows: guide.rows.into_iter().map(guide_key_row).collect(),
        }
    }
}

impl App {
    pub(super) fn sync_keyguide(&mut self) -> bool {
        let now = Instant::now();
        if !self.keyguide_enabled {
            return self.clear_keyguide();
        }
        if self.pending_leader_path().is_some() {
            self.keyguide_cache = None;
            return self.update_keyguide_visibility(now);
        }
        let mode = self.editor.mode();
        let Some(prefix) = self.keymap.pending_keys() else {
            return self.clear_keyguide();
        };
        if !self
            .keyguide_cache
            .as_ref()
            .is_some_and(|cache| cache.prefix == prefix && cache.mode == mode)
        {
            let Some(guide) = guide::guide(prefix, mode) else {
                return self.clear_keyguide();
            };
            self.keyguide_cache = Some(KeyGuideCache::new(prefix, mode, guide));
        }
        self.update_keyguide_visibility(now)
    }

    pub(super) fn pending_leader_path(&self) -> Option<LeaderPath> {
        // TODO: unify built-in leader sequences with the keymap resolver when expanding the plugin API.
        self.keymap
            .leader_path()
            .or_else(|| self.resolver_leader_path())
    }

    fn resolver_leader_path(&self) -> Option<LeaderPath> {
        let mode = self.editor.mode();
        if !mode.is_normal_state() {
            return None;
        }
        let context = KeyContext::from_mode(mode)?;
        if self.keymaps.pending_context() != Some(context) {
            return None;
        }
        let (leader, rest) = self.keymaps.pending_chords().split_first()?;
        if !self.leaders.contains(leader) {
            return None;
        }
        let Some((group_key, _)) = rest.split_first() else {
            return Some(LeaderPath::Root);
        };
        let group = leader::group(plain_chord(*group_key)?)?;
        if mode.is_visual() && !group.visual {
            return None;
        }
        Some(LeaderPath::Group(group))
    }

    pub(super) fn keyguide_wait_timeout(&self) -> Option<Duration> {
        if !self.keyguide_enabled || self.keyguide_visible {
            return None;
        }
        let since = self.keyguide_since?;
        Some(self.keyguide_delay.saturating_sub(since.elapsed()))
    }

    fn update_keyguide_visibility(&mut self, now: Instant) -> bool {
        if self.keyguide_since.is_none() {
            self.keyguide_since = Some(now);
        }
        let since = self.keyguide_since.unwrap_or(now);
        let visible =
            self.keyguide_delay.is_zero() || now.duration_since(since) >= self.keyguide_delay;
        let changed = visible != self.keyguide_visible;
        self.keyguide_visible = visible;
        changed
    }

    fn clear_keyguide(&mut self) -> bool {
        let changed = self.keyguide_visible;
        self.keyguide_since = None;
        self.keyguide_visible = false;
        self.keyguide_cache = None;
        changed
    }
}

fn plain_chord(chord: KeyChord) -> Option<char> {
    if chord.ctrl || chord.alt {
        return None;
    }
    match chord.code {
        KeyCodeSpec::Char(character) => Some(character),
        _ => None,
    }
}
