use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub(crate) struct TreeLink {
    pub(crate) id: u64,
    pub(crate) parent: Option<u64>,
    pub(crate) timestamp: u64,
}

pub(crate) struct Removal {
    pub(crate) ids: HashSet<u64>,
    pub(crate) parent: Option<u64>,
    pub(crate) reroot_child: Option<u64>,
}

pub(crate) fn plan_removal(links: &[TreeLink], current_id: u64) -> Option<Removal> {
    let current_path = current_path(links, current_id)?;
    if let Some(root) = off_path_candidate(links, &current_path) {
        return Some(Removal {
            ids: subtree_ids(links, root.id),
            parent: root.parent,
            reroot_child: None,
        });
    }
    if let Some((root, child)) = reroot_candidate(links, current_id) {
        return Some(Removal {
            ids: HashSet::from([root]),
            parent: None,
            reroot_child: Some(child),
        });
    }
    let leaf = oldest_leaf(links, current_id)?;
    Some(Removal {
        ids: HashSet::from([leaf.id]),
        parent: leaf.parent,
        reroot_child: None,
    })
}

fn off_path_candidate<'a>(
    links: &'a [TreeLink],
    current_path: &HashSet<u64>,
) -> Option<&'a TreeLink> {
    links
        .iter()
        .filter(|link| {
            link.parent
                .is_some_and(|parent| current_path.contains(&parent))
                && !current_path.contains(&link.id)
        })
        .min_by_key(|link| (link.timestamp, link.id))
}

fn reroot_candidate(links: &[TreeLink], current_id: u64) -> Option<(u64, u64)> {
    let root = links.iter().find(|link| link.parent.is_none())?;
    if root.id == current_id {
        return None;
    }
    let child = links.iter().find(|link| link.parent == Some(root.id))?;
    Some((root.id, child.id))
}

fn oldest_leaf(links: &[TreeLink], current_id: u64) -> Option<&TreeLink> {
    let parents: HashSet<u64> = links.iter().filter_map(|link| link.parent).collect();
    links
        .iter()
        .filter(|link| link.id != current_id && !parents.contains(&link.id))
        .min_by_key(|link| (link.timestamp, link.id))
}

fn current_path(links: &[TreeLink], current_id: u64) -> Option<HashSet<u64>> {
    let by_id: HashMap<u64, TreeLink> = links.iter().map(|link| (link.id, *link)).collect();
    let mut path = HashSet::new();
    let mut current = current_id;
    loop {
        if !path.insert(current) {
            return None;
        }
        let link = by_id.get(&current)?;
        let Some(parent) = link.parent else {
            return Some(path);
        };
        if !by_id.contains_key(&parent) {
            return None;
        }
        current = parent;
    }
}

fn subtree_ids(links: &[TreeLink], root: u64) -> HashSet<u64> {
    let mut ids = HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !ids.insert(id) {
            continue;
        }
        stack.extend(
            links
                .iter()
                .filter(|link| link.parent == Some(id))
                .map(|link| link.id),
        );
    }
    ids
}
