use kawi_tui::frame::Rect;

use super::{FocusDirection, ResizeAxis, SplitDirection, WindowId};

const MIN_PANE_WIDTH: usize = 8;
const MIN_PANE_HEIGHT: usize = 3;
const MIN_RATIO: u16 = 10;
const MAX_RATIO: u16 = 90;
const RESIZE_STEP: u16 = 5;

pub(super) enum Node {
    Leaf(WindowId),
    Split {
        direction: SplitDirection,
        ratio: u16,
        first: Box<Node>,
        second: Box<Node>,
    },
}

pub struct WindowLayout {
    pub panes: Vec<(WindowId, Rect)>,
    pub separators: Vec<Rect>,
}

impl WindowLayout {
    pub(super) fn compute(root: Option<&Node>, area: Rect) -> Self {
        let mut layout = Self {
            panes: Vec::new(),
            separators: Vec::new(),
        };
        if let Some(root) = root {
            layout_node(root, area, &mut layout);
        }
        layout
    }
}

pub(super) fn split_leaf(
    node: &mut Node,
    target: WindowId,
    id: WindowId,
    direction: SplitDirection,
) -> bool {
    match node {
        Node::Leaf(leaf) if *leaf == target => {
            let first = Box::new(Node::Leaf(target));
            let second = Box::new(Node::Leaf(id));
            *node = Node::Split {
                direction,
                ratio: 50,
                first,
                second,
            };
            true
        }
        Node::Leaf(_) => false,
        Node::Split { first, second, .. } => {
            if contains(first, target) {
                split_leaf(first, target, id, direction)
            } else if contains(second, target) {
                split_leaf(second, target, id, direction)
            } else {
                false
            }
        }
    }
}

pub(super) fn remove_leaf(node: Node, target: WindowId) -> (Option<Node>, Option<WindowId>) {
    match node {
        Node::Leaf(id) if id == target => (None, None),
        Node::Leaf(id) => (Some(Node::Leaf(id)), None),
        Node::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            if contains(&first, target) {
                let (first, focus) = remove_leaf(*first, target);
                match first {
                    Some(first) => (
                        Some(Node::Split {
                            direction,
                            ratio,
                            first: Box::new(first),
                            second,
                        }),
                        focus,
                    ),
                    None => {
                        let focus = focus.unwrap_or_else(|| first_leaf(&second));
                        (Some(*second), Some(focus))
                    }
                }
            } else {
                let (second, focus) = remove_leaf(*second, target);
                match second {
                    Some(second) => (
                        Some(Node::Split {
                            direction,
                            ratio,
                            first,
                            second: Box::new(second),
                        }),
                        focus,
                    ),
                    None => {
                        let focus = focus.unwrap_or_else(|| first_leaf(&first));
                        (Some(*first), Some(focus))
                    }
                }
            }
        }
    }
}

pub(super) fn contains(node: &Node, id: WindowId) -> bool {
    match node {
        Node::Leaf(leaf) => *leaf == id,
        Node::Split { first, second, .. } => contains(first, id) || contains(second, id),
    }
}

pub(super) fn first_leaf(node: &Node) -> WindowId {
    match node {
        Node::Leaf(id) => *id,
        Node::Split { first, .. } => first_leaf(first),
    }
}

fn layout_node(node: &Node, area: Rect, layout: &mut WindowLayout) {
    match node {
        Node::Leaf(id) => layout.panes.push((*id, area)),
        Node::Split {
            direction,
            ratio,
            first,
            second,
        } => match direction {
            SplitDirection::Vertical => {
                let usable = area.width.saturating_sub(1);
                let first_width = split_size(usable, *ratio, MIN_PANE_WIDTH);
                let second_width = usable.saturating_sub(first_width);
                layout_node(
                    first,
                    Rect::new(area.x, area.y, first_width, area.height),
                    layout,
                );
                if area.width > 1 {
                    layout
                        .separators
                        .push(Rect::new(area.x + first_width, area.y, 1, area.height));
                }
                layout_node(
                    second,
                    Rect::new(
                        area.x + first_width + usize::from(area.width > 1),
                        area.y,
                        second_width,
                        area.height,
                    ),
                    layout,
                );
            }
            SplitDirection::Horizontal => {
                let first_height = split_size(area.height, *ratio, MIN_PANE_HEIGHT);
                let second_height = area.height.saturating_sub(first_height);
                layout_node(
                    first,
                    Rect::new(area.x, area.y, area.width, first_height),
                    layout,
                );
                layout_node(
                    second,
                    Rect::new(area.x, area.y + first_height, area.width, second_height),
                    layout,
                );
            }
        },
    }
}

fn split_size(total: usize, ratio: u16, minimum: usize) -> usize {
    if total >= minimum.saturating_mul(2) {
        ((total.saturating_mul(usize::from(ratio))).saturating_add(50) / 100)
            .clamp(minimum, total - minimum)
    } else {
        total / 2
    }
}

pub(super) fn normalized_ratio(ratio: u16) -> u16 {
    ratio.clamp(MIN_RATIO, MAX_RATIO)
}

#[cfg(test)]
pub(super) fn pane_rect(panes: &[(WindowId, Rect)], id: WindowId) -> Option<Rect> {
    panes
        .iter()
        .find_map(|(candidate, rect)| (*candidate == id).then_some(*rect))
}

pub(crate) fn direction_score(
    direction: FocusDirection,
    current: Rect,
    candidate: Rect,
) -> Option<(usize, usize, usize)> {
    let (gap, overlap) = match direction {
        FocusDirection::Left => {
            if candidate.right() > current.x {
                return None;
            }
            (
                current.x.saturating_sub(candidate.right()),
                vertical_overlap(current, candidate),
            )
        }
        FocusDirection::Right => {
            if candidate.x < current.right() {
                return None;
            }
            (
                candidate.x.saturating_sub(current.right()),
                vertical_overlap(current, candidate),
            )
        }
        FocusDirection::Up => {
            if candidate.bottom() > current.y {
                return None;
            }
            (
                current.y.saturating_sub(candidate.bottom()),
                horizontal_overlap(current, candidate),
            )
        }
        FocusDirection::Down => {
            if candidate.y < current.bottom() {
                return None;
            }
            (
                candidate.y.saturating_sub(current.bottom()),
                horizontal_overlap(current, candidate),
            )
        }
    };
    let dx = current.x.abs_diff(candidate.x);
    let dy = current.y.abs_diff(candidate.y);
    Some((gap, overlap, dx.saturating_add(dy)))
}

pub(crate) fn score_precedes(
    candidate: (usize, usize, usize),
    current: (usize, usize, usize),
) -> bool {
    candidate.0 < current.0
        || (candidate.0 == current.0
            && (candidate.1 > current.1 || (candidate.1 == current.1 && candidate.2 < current.2)))
}

fn vertical_overlap(first: Rect, second: Rect) -> usize {
    overlap(first.y, first.bottom(), second.y, second.bottom())
}

fn horizontal_overlap(first: Rect, second: Rect) -> usize {
    overlap(first.x, first.right(), second.x, second.right())
}

fn overlap(first_start: usize, first_end: usize, second_start: usize, second_end: usize) -> usize {
    first_end
        .min(second_end)
        .saturating_sub(first_start.max(second_start))
}

pub(super) fn adjust_active(
    node: &mut Node,
    target: WindowId,
    axis: ResizeAxis,
    grow: bool,
) -> bool {
    match node {
        Node::Leaf(_) => false,
        Node::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let in_second = contains(second, target);
            if !in_second && !contains(first, target) {
                return false;
            }
            let branch = if in_second { second } else { first };
            if adjust_active(branch, target, axis, grow) {
                return true;
            }
            if split_axis(*direction) != axis {
                return false;
            }
            let before = *ratio;
            let first_grows = (!in_second && grow) || (in_second && !grow);
            let updated = if first_grows {
                ratio.saturating_add(RESIZE_STEP)
            } else {
                ratio.saturating_sub(RESIZE_STEP)
            };
            *ratio = updated.clamp(MIN_RATIO, MAX_RATIO);
            *ratio != before
        }
    }
}

fn split_axis(direction: SplitDirection) -> ResizeAxis {
    match direction {
        SplitDirection::Horizontal => ResizeAxis::Height,
        SplitDirection::Vertical => ResizeAxis::Width,
    }
}

pub(super) fn equalize_node(node: &mut Node) -> bool {
    match node {
        Node::Leaf(_) => false,
        Node::Split {
            ratio,
            first,
            second,
            ..
        } => {
            let changed = *ratio != 50;
            *ratio = 50;
            equalize_node(first) | equalize_node(second) | changed
        }
    }
}
