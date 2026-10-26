use crossterm::event::KeyEvent;
use kawi_editor::Action;

use super::{plain_char, VimKeymap};
use crate::leader::{self, LeaderPath};

impl VimKeymap {
    pub(super) fn leader_prefix_armed(&self) -> bool {
        self.pending_leader_path().is_some()
    }

    pub(super) fn pending_leader_path(&self) -> Option<LeaderPath> {
        if self.leader_space {
            return Some(LeaderPath::Root);
        }
        let mut pending = self.pending.chars();
        if !matches!(pending.next(), Some(' ' | '\\')) {
            return None;
        }
        let rest = pending.as_str();
        if rest.is_empty() {
            return Some(LeaderPath::Root);
        }
        let mut group = rest.chars();
        let key = group.next()?;
        if group.next().is_some() {
            return None;
        }
        leader::group(key).map(LeaderPath::Group)
    }

    pub(super) fn resolve_leader_pending(&mut self, key: KeyEvent) -> Action {
        let Some(path) = self.pending_leader_path() else {
            return self.cancel_sequence();
        };
        let Some(pressed) = plain_char(key) else {
            return self.cancel_sequence();
        };
        match path {
            LeaderPath::Root => self.enter_leader_group(pressed),
            LeaderPath::Group(_) => {
                let Some(action) = leader::action(path, pressed) else {
                    return self.cancel_sequence();
                };
                self.clear_pending();
                action
            }
        }
    }

    fn enter_leader_group(&mut self, key: char) -> Action {
        if leader::group(key).is_none() {
            return self.cancel_sequence();
        }
        if self.leader_space {
            self.leader_space = false;
            self.pending.push(' ');
        }
        self.pending.push(key);
        Action::Noop
    }
}
