use std::collections::BTreeMap;

use crate::session::{SessionDirection, SessionView, SessionWindow, MAX_WINDOW_DEPTH};

use super::layout::{first_leaf, normalized_ratio, Node};
use super::{SplitDirection, WindowId, WindowView, Windows};

impl Windows {
    pub(crate) fn snapshot(
        &self,
        buffer_indices: &BTreeMap<usize, usize>,
    ) -> Option<SessionWindow> {
        snapshot_node(self.root.as_ref()?, buffer_indices, &self.windows, 1)
    }

    pub(crate) fn from_session(
        root: &SessionWindow,
        active: WindowId,
        buffers: &[Option<usize>],
    ) -> Option<Self> {
        let mut windows = BTreeMap::new();
        let root = restore_node(root, buffers, &mut windows, 1).ok()??;
        let active = if windows.contains_key(&active) {
            active
        } else {
            first_leaf(&root)
        };
        let next_id = windows.keys().next_back()?.checked_add(1)?;
        Some(Self {
            root: Some(root),
            windows,
            active,
            next_id,
        })
    }
}

fn snapshot_node(
    node: &Node,
    buffer_indices: &BTreeMap<usize, usize>,
    windows: &BTreeMap<WindowId, WindowView>,
    depth: usize,
) -> Option<SessionWindow> {
    if depth > MAX_WINDOW_DEPTH {
        return None;
    }
    match node {
        Node::Leaf(id) => {
            let window = windows.get(id)?;
            let buffer = *buffer_indices.get(&window.buffer)?;
            Some(SessionWindow::Leaf {
                id: *id,
                buffer,
                view: window.view.into(),
            })
        }
        Node::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let first = snapshot_node(first, buffer_indices, windows, depth + 1);
            let second = snapshot_node(second, buffer_indices, windows, depth + 1);
            match (first, second) {
                (Some(first), Some(second)) => Some(SessionWindow::Split {
                    direction: session_direction(*direction),
                    ratio: *ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(first), None) => Some(first),
                (None, Some(second)) => Some(second),
                (None, None) => None,
            }
        }
    }
}

fn restore_node(
    window: &SessionWindow,
    buffers: &[Option<usize>],
    windows: &mut BTreeMap<WindowId, WindowView>,
    depth: usize,
) -> Result<Option<Node>, ()> {
    if depth > MAX_WINDOW_DEPTH {
        return Err(());
    }
    match window {
        SessionWindow::Leaf { id, buffer, view } => {
            if windows.contains_key(id) {
                return Err(());
            }
            let buffer = *buffers.get(*buffer).ok_or(())?;
            let Some(buffer) = buffer else {
                return Ok(None);
            };
            windows.insert(*id, WindowView::new(buffer, SessionView::into(*view)));
            Ok(Some(Node::Leaf(*id)))
        }
        SessionWindow::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let first = restore_node(first, buffers, windows, depth + 1)?;
            let second = restore_node(second, buffers, windows, depth + 1)?;
            Ok(match (first, second) {
                (Some(first), Some(second)) => Some(Node::Split {
                    direction: window_direction(*direction),
                    ratio: normalized_ratio(*ratio),
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(first), None) => Some(first),
                (None, Some(second)) => Some(second),
                (None, None) => None,
            })
        }
    }
}

fn session_direction(direction: SplitDirection) -> SessionDirection {
    match direction {
        SplitDirection::Horizontal => SessionDirection::Horizontal,
        SplitDirection::Vertical => SessionDirection::Vertical,
    }
}

fn window_direction(direction: SessionDirection) -> SplitDirection {
    match direction {
        SessionDirection::Horizontal => SplitDirection::Horizontal,
        SessionDirection::Vertical => SplitDirection::Vertical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_editor::ViewState;
    use kawi_tui::frame::Rect;

    fn view(row: usize, top: usize) -> ViewState {
        ViewState {
            row,
            col: 1,
            desired: 2,
            top,
            leftcol: 3,
        }
    }

    fn session_view(row: usize, top: usize) -> SessionView {
        view(row, top).into()
    }

    fn area() -> Rect {
        Rect::new(0, 0, 100, 40)
    }

    #[test]
    fn snapshot_round_trips_the_window_tree() {
        let mut windows = Windows::new(1, view(2, 1));
        let right = windows.split_active(SplitDirection::Vertical, WindowView::new(2, view(0, 0)));
        let bottom =
            windows.split_active(SplitDirection::Horizontal, WindowView::new(1, view(4, 3)));
        let mut indices = BTreeMap::new();
        indices.insert(1, 0);
        indices.insert(2, 1);
        let snapshot = windows.snapshot(&indices).expect("snapshot");
        let restored = Windows::from_session(&snapshot, windows.active(), &[Some(1), Some(2)])
            .expect("restore");
        assert_eq!(restored.len(), 3);
        assert_eq!(restored.active(), windows.active());
        assert_eq!(restored.layout(area()).panes, windows.layout(area()).panes);
        assert_eq!(
            restored.layout(area()).separators,
            windows.layout(area()).separators
        );
        assert_eq!(restored.view(right), windows.view(right));
        assert_eq!(restored.view(bottom), windows.view(bottom));
        assert_eq!(restored.view(1), windows.view(1));
    }

    #[test]
    fn unlisted_buffers_are_pruned_from_the_snapshot() {
        let mut windows = Windows::new(1, view(0, 0));
        windows.split_active(SplitDirection::Vertical, WindowView::new(2, view(0, 0)));
        let mut indices = BTreeMap::new();
        indices.insert(1, 0);
        let snapshot = windows.snapshot(&indices).expect("snapshot");
        assert!(matches!(snapshot, SessionWindow::Leaf { buffer: 0, .. }));
    }

    #[test]
    fn missing_buffers_prune_windows_and_fallback_focus() {
        let snapshot = SessionWindow::Split {
            direction: SessionDirection::Vertical,
            ratio: 50,
            first: Box::new(SessionWindow::Leaf {
                id: 1,
                buffer: 0,
                view: session_view(0, 0),
            }),
            second: Box::new(SessionWindow::Leaf {
                id: 2,
                buffer: 1,
                view: session_view(0, 0),
            }),
        };
        let restored = Windows::from_session(&snapshot, 2, &[Some(7), None]).expect("restore");
        assert_eq!(restored.len(), 1);
        assert_eq!(restored.active(), 1);
        assert_eq!(restored.active_view().buffer, 7);
        assert_eq!(restored.layout(area()).panes.len(), 1);
    }

    #[test]
    fn duplicate_window_ids_are_rejected() {
        let snapshot = SessionWindow::Split {
            direction: SessionDirection::Horizontal,
            ratio: 50,
            first: Box::new(SessionWindow::Leaf {
                id: 1,
                buffer: 0,
                view: session_view(0, 0),
            }),
            second: Box::new(SessionWindow::Leaf {
                id: 1,
                buffer: 0,
                view: session_view(0, 0),
            }),
        };
        assert!(Windows::from_session(&snapshot, 1, &[Some(1)]).is_none());
    }

    #[test]
    fn invalid_ratios_are_clamped() {
        let snapshot = SessionWindow::Split {
            direction: SessionDirection::Vertical,
            ratio: 0,
            first: Box::new(SessionWindow::Leaf {
                id: 1,
                buffer: 0,
                view: session_view(0, 0),
            }),
            second: Box::new(SessionWindow::Leaf {
                id: 2,
                buffer: 0,
                view: session_view(0, 0),
            }),
        };
        let restored = Windows::from_session(&snapshot, 1, &[Some(1)]).expect("restore");
        let panes = restored.layout(area()).panes;
        assert_eq!(panes[0].1.width, 10);
        assert_eq!(panes[1].1.width, 89);
    }
}
