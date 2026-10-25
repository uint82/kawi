use super::history::{History, MAX_HISTORY};
use crate::history_tree::{plan_removal, TreeLink};

impl History {
    pub(crate) fn evict_to_cap(&mut self) {
        while self.nodes.len() > MAX_HISTORY {
            if !self.evict_one() {
                break;
            }
        }
    }

    fn evict_one(&mut self) -> bool {
        let links: Vec<TreeLink> = self
            .nodes
            .iter()
            .map(|(id, node)| TreeLink {
                id: *id,
                parent: node.parent,
                timestamp: node.timestamp,
            })
            .collect();
        let Some(removal) = plan_removal(&links, self.current_id) else {
            return false;
        };
        for id in &removal.ids {
            self.nodes.remove(id);
        }
        if self.clean_id.is_some_and(|id| removal.ids.contains(&id)) {
            self.clean_id = None;
        }
        if let Some(parent) = removal.parent {
            if let Some(node) = self.nodes.get_mut(&parent) {
                if node
                    .last_visited_child
                    .is_some_and(|id| removal.ids.contains(&id))
                {
                    node.last_visited_child = None;
                }
            }
        }
        if let Some(child) = removal.reroot_child {
            if let Some(node) = self.nodes.get_mut(&child) {
                node.parent = None;
            }
        }
        true
    }
}
