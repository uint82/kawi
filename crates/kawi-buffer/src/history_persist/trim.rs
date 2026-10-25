use super::{HistoryDelta, PersistedHistory};
use crate::history_tree::{plan_removal, TreeLink};

impl PersistedHistory {
    pub fn trim_oldest(&mut self) -> bool {
        let links: Vec<TreeLink> = self
            .entries
            .iter()
            .map(|entry| TreeLink {
                id: entry.id,
                parent: entry.parent,
                timestamp: entry.timestamp,
            })
            .collect();
        let Some(removal) = plan_removal(&links, self.current_id) else {
            return false;
        };
        self.entries
            .retain(|entry| !removal.ids.contains(&entry.id));
        if self.clean_id.is_some_and(|id| removal.ids.contains(&id)) {
            self.clean_id = None;
        }
        if let Some(parent) = removal.parent {
            if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == parent) {
                if entry
                    .last_visited_child
                    .is_some_and(|id| removal.ids.contains(&id))
                {
                    entry.last_visited_child = None;
                }
            }
        }
        if let Some(child) = removal.reroot_child {
            if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == child) {
                entry.parent = None;
                entry.delta = HistoryDelta::default();
            }
        }
        true
    }
}
