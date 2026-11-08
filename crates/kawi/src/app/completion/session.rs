use std::collections::{BTreeMap, BTreeSet};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::TextEdit;
use kawi_completion::{Documentation, Event, Item, ItemId, RequestId};
use kawi_syntax::LanguageId;
use kawi_tui::frame::CompletionLine;
use kawi_tui::markdown::MarkdownDocument;
use kawi_tui::overlay::DocsSide;

use super::super::lsp_menu::MenuSelection;
use super::super::overlay::document::prepare_documentation;
use session_display::{display_line, metrics};
use session_filter::{filter, FilteredItem};

mod session_display;
mod session_edit;
mod session_filter;
mod session_keys;
mod session_resolve;

#[cfg(test)]
mod tests;

pub(crate) struct CompletionSession {
    pub(super) event: Event,
    pub(super) items: Vec<Item>,
    pub(super) filtered: Vec<FilteredItem>,
    pub(super) displays: Vec<CompletionLine>,
    pub(super) selection: MenuSelection,
    pub(super) docs: BTreeMap<ItemId, Documentation>,
    pub(super) prepared: BTreeMap<ItemId, MarkdownDocument>,
    pub(super) additional: BTreeMap<ItemId, Vec<TextEdit>>,
    pub(super) resolved: BTreeSet<ItemId>,
    pub(super) resolve_pending: Option<(RequestId, ItemId)>,
    pub(super) resolve_failed: BTreeSet<ItemId>,
    pub(super) docs_scroll: usize,
    pub(super) docs_side: Option<DocsSide>,
    pub(super) docs_width: usize,
    pub(super) list_width: usize,
    pub(super) kind_width: usize,
    fallback_language: Option<LanguageId>,
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
        let display = metrics(&items);
        let fallback_language = event.path.to_str().and_then(LanguageId::from_path);
        let mut session = Self {
            event,
            items,
            filtered: Vec::new(),
            displays: Vec::new(),
            selection: MenuSelection::new(0, None),
            docs: BTreeMap::new(),
            prepared: BTreeMap::new(),
            additional: BTreeMap::new(),
            resolved: BTreeSet::new(),
            resolve_pending: None,
            resolve_failed: BTreeSet::new(),
            docs_scroll: 0,
            docs_side: None,
            docs_width: 0,
            list_width: display.list_width,
            kind_width: display.kind_width,
            fallback_language,
        };
        session.refilter(query, selected.map(|index| session.items[index].id.clone()));
        session
    }

    pub(super) fn update(&mut self, event: Event, query: &str) {
        let selected = self.selected_id();
        self.event = event;
        self.fallback_language = self.event.path.to_str().and_then(LanguageId::from_path);
        self.items = self.event.items.clone();
        let display = metrics(&self.items);
        self.list_width = display.list_width;
        self.kind_width = display.kind_width;
        self.docs.clear();
        self.prepared.clear();
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

    pub(crate) fn items(&self) -> &[CompletionLine] {
        &self.displays
    }

    pub(crate) fn selected(&self) -> Option<usize> {
        self.selection.selected()
    }

    pub(super) fn is_incomplete(&self) -> bool {
        self.event.is_incomplete
    }

    pub(crate) fn list_width(&self) -> usize {
        self.list_width
    }

    pub(crate) fn kind_width(&self) -> usize {
        self.kind_width
    }

    pub(super) fn selected_item(&self) -> Option<&Item> {
        let filtered = self.filtered.get(self.selection.selected()?)?;
        self.items.get(filtered.index)
    }

    fn selected_id(&self) -> Option<ItemId> {
        self.selected_item().map(|item| item.id.clone())
    }

    pub(crate) fn selected_document(&self) -> Option<&MarkdownDocument> {
        self.prepared.get(&self.selected_item()?.id)
    }

    pub(super) fn prepare_selected_document(&mut self) {
        let Some(id) = self.selected_item().map(|item| item.id.clone()) else {
            return;
        };
        self.prepare_document(&id);
    }

    pub(super) fn prepare_document(&mut self, id: &ItemId) {
        if self.prepared.contains_key(id) {
            return;
        }
        let documentation = self
            .docs
            .get(id)
            .or_else(|| {
                self.items
                    .iter()
                    .find(|item| item.id == *id)?
                    .documentation
                    .as_ref()
            })
            .filter(|documentation| !documentation.text.trim().is_empty())
            .cloned();
        let Some(documentation) = documentation else {
            return;
        };
        let document = prepare_documentation(&documentation, self.fallback_language);
        self.prepared.insert(id.clone(), document);
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

    pub(crate) fn docs_width(&self) -> usize {
        self.docs_width
    }

    pub(crate) fn grow_docs_width(&mut self, width: usize) {
        self.docs_width = self.docs_width.max(width);
    }

    pub(super) fn refilter(&mut self, query: &str, preferred: Option<ItemId>) {
        self.filtered = filter(&self.items, query);
        let preferred = preferred
            .and_then(|preferred| {
                self.filtered
                    .iter()
                    .position(|filtered| self.items[filtered.index].id == preferred)
            })
            .or_else(|| {
                self.filtered
                    .iter()
                    .position(|filtered| self.items[filtered.index].preselect)
            });
        self.selection = MenuSelection::new(self.filtered.len(), preferred);
        self.rebuild_displays();
        self.prepare_selected_document();
    }

    pub(super) fn rebuild_displays(&mut self) {
        self.displays = self
            .filtered
            .iter()
            .filter_map(|filtered| {
                self.items
                    .get(filtered.index)
                    .map(|item| display_line(item, &filtered.highlights))
            })
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
