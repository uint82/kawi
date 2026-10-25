use kawi_completion::{Item, RequestId, ResolvedItem};

use super::CompletionSession;

impl CompletionSession {
    pub(crate) fn needs_resolve(&self) -> Option<&Item> {
        let item = self.selected_item()?;
        if !item.resolve
            || self.resolved.contains(&item.id)
            || self.resolve_failed.contains(&item.id)
            || self
                .resolve_pending
                .as_ref()
                .is_some_and(|(_, item_id)| item_id == &item.id)
        {
            return None;
        }
        Some(item)
    }

    pub(crate) fn start_resolve(&mut self, request: RequestId, item: &Item) {
        self.resolve_pending = Some((request, item.id.clone()));
    }

    pub(crate) fn apply_resolved(&mut self, resolved: ResolvedItem) -> bool {
        if !self
            .resolve_pending
            .as_ref()
            .is_some_and(|(request, item_id)| {
                *request == resolved.request && item_id == &resolved.item_id
            })
        {
            return false;
        }
        self.resolve_pending = None;
        if !self.items.iter().any(|item| item.id == resolved.item_id) {
            return false;
        }
        let item_id = resolved.item_id.clone();
        self.resolved.insert(item_id.clone());
        if let Some(documentation) = resolved.documentation {
            self.docs.insert(item_id.clone(), documentation);
            self.prepared.remove(&item_id);
            self.prepare_document(&item_id);
        }
        if !resolved.additional.is_empty() {
            self.additional
                .insert(resolved.item_id.clone(), resolved.additional);
        }
        if let Some(detail) = resolved.detail {
            if let Some(item) = self
                .items
                .iter_mut()
                .find(|item| item.id == resolved.item_id)
            {
                item.detail = Some(detail);
            }
            self.rebuild_displays();
        }
        true
    }

    pub(crate) fn resolve_failed(&mut self, request: RequestId) -> bool {
        let Some((pending, item_id)) = self.resolve_pending.take() else {
            return false;
        };
        if pending != request {
            self.resolve_pending = Some((pending, item_id));
            return false;
        }
        self.resolve_failed.insert(item_id);
        true
    }
}
