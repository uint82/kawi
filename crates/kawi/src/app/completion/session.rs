use std::collections::{BTreeMap, BTreeSet};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::TextEdit;
use kawi_completion::{Documentation, Event, Item, ItemId, RequestId};
use kawi_picker::fuzzy_score;
use kawi_tui::overlay::DocsSide;

use super::super::lsp_menu::MenuSelection;
use session_display::{display_item, list_width};

mod session_display;
mod session_edit;
mod session_keys;
mod session_resolve;

#[cfg(test)]
mod tests;

pub(crate) struct CompletionSession {
    pub(super) event: Event,
    pub(super) items: Vec<Item>,
    pub(super) filtered: Vec<usize>,
    pub(super) displays: Vec<String>,
    pub(super) selection: MenuSelection,
    pub(super) docs: BTreeMap<ItemId, Documentation>,
    pub(super) additional: BTreeMap<ItemId, Vec<TextEdit>>,
    pub(super) resolved: BTreeSet<ItemId>,
    pub(super) resolve_pending: Option<(RequestId, ItemId)>,
    pub(super) resolve_failed: BTreeSet<ItemId>,
    pub(super) docs_scroll: usize,
    pub(super) docs_side: Option<DocsSide>,
    pub(super) list_width: usize,
}

pub(crate) enum CompletionOutcome {
    Consumed,
    Cancel,
    Accept,
    Fallthrough,
}

impl CompletionSession {
    pub(super) fn new(event: Event, query: &str) -> Self {
        let items = event.items.clone();
        let selected = items.iter().position(|item| item.preselect);
        let list_width = list_width(&items);
        let mut session = Self {
            event,
            items,
            filtered: Vec::new(),
            displays: Vec::new(),
            selection: MenuSelection::new(0, None),
            docs: BTreeMap::new(),
            additional: BTreeMap::new(),
            resolved: BTreeSet::new(),
            resolve_pending: None,
            resolve_failed: BTreeSet::new(),
            docs_scroll: 0,
            docs_side: None,
            list_width,
        };
        session.refilter(query, selected.map(|index| session.items[index].id.clone()));
        session
    }

    pub(super) fn update(&mut self, event: Event, query: &str) {
        let selected = self.selected_id();
        self.event = event;
        self.items = self.event.items.clone();
        self.list_width = list_width(&self.items);
        self.docs.clear();
        self.additional.clear();
        self.resolved.clear();
        self.resolve_pending = None;
        self.resolve_failed.clear();
        self.docs_scroll = 0;
        let preferred = self.items.iter().position(|item| item.preselect);
        self.refilter(
            query,
            selected.or_else(|| preferred.map(|index| self.items[index].id.clone())),
        );
    }

    pub(crate) fn items(&self) -> &[String] {
        &self.displays
    }

    pub(crate) fn selected(&self) -> Option<usize> {
        self.selection.selected()
    }

    pub(crate) fn list_width(&self) -> usize {
        self.list_width
    }

    pub(super) fn selected_item(&self) -> Option<&Item> {
        self.filtered
            .get(self.selection.selected()?)
            .and_then(|index| self.items.get(*index))
    }

    fn selected_id(&self) -> Option<ItemId> {
        self.selected_item().map(|item| item.id.clone())
    }

    pub(crate) fn selected_documentation(&self) -> Option<&Documentation> {
        let item = self.selected_item()?;
        self.docs
            .get(&item.id)
            .or(item.documentation.as_ref())
            .filter(|documentation| !documentation.text.trim().is_empty())
    }

    pub(crate) fn docs_scroll(&self) -> usize {
        self.docs_scroll
    }

    pub(crate) fn clamp_docs_scroll(&mut self, limit: usize) {
        self.docs_scroll = self.docs_scroll.min(limit);
    }

    pub(super) fn scroll_docs(&mut self, delta: isize, limit: usize) {
        self.docs_scroll = self.docs_scroll.saturating_add_signed(delta).min(limit);
    }

    pub(crate) fn docs_side(&self) -> Option<DocsSide> {
        self.docs_side
    }

    pub(crate) fn set_docs_side(&mut self, side: DocsSide) {
        self.docs_side = Some(side);
    }

    pub(super) fn refilter(&mut self, query: &str, preferred: Option<ItemId>) {
        let mut scores = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            if let Some(score) = fuzzy_score(query, item.filter_key()) {
                scores.push((index, score));
            }
        }
        scores.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| {
                    self.items[left.0]
                        .sort_key()
                        .cmp(self.items[right.0].sort_key())
                })
                .then_with(|| left.0.cmp(&right.0))
        });
        self.filtered = scores.into_iter().map(|(index, _)| index).collect();
        let preferred = preferred
            .and_then(|preferred| {
                self.filtered
                    .iter()
                    .position(|index| self.items[*index].id == preferred)
            })
            .or_else(|| {
                self.filtered
                    .iter()
                    .position(|index| self.items[*index].preselect)
            });
        self.selection = MenuSelection::new(self.filtered.len(), preferred);
        self.rebuild_displays();
    }

    pub(super) fn rebuild_displays(&mut self) {
        self.displays = self
            .filtered
            .iter()
            .filter_map(|index| self.items.get(*index))
            .map(display_item)
            .collect();
    }

    pub(super) fn command(&self) -> Option<&kawi_completion::Command> {
        self.selected_item()?.command.as_ref()
    }

    pub(super) fn path(&self) -> &std::path::Path {
        &self.event.path
    }
}

pub(super) fn is_continuation_key(key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return false;
    }
    match key.code {
        KeyCode::Backspace => true,
        KeyCode::Char(character) => character.is_alphanumeric() || character == '_',
        _ => false,
    }
}
