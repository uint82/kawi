use crate::frame::Rect;

const GAP: usize = 1;
const MIN_DOCS_WIDTH: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsSide {
    Right,
    Left,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

impl Size {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementInput {
    pub area: Rect,
    pub anchor: (usize, usize),
    pub list: Size,
    pub docs: Option<Size>,
    pub preferred_docs_side: Option<DocsSide>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub list: Rect,
    pub docs: Option<Rect>,
    pub docs_side: Option<DocsSide>,
}

pub fn place(input: PlacementInput) -> Placement {
    let list = place_list(input.area, input.anchor, input.list);
    let Some(docs) = input.docs else {
        return Placement {
            list,
            docs: None,
            docs_side: None,
        };
    };
    let (docs, docs_side) = place_docs(input, list, docs);
    Placement {
        list,
        docs,
        docs_side,
    }
}

fn place_list(area: Rect, anchor: (usize, usize), size: Size) -> Rect {
    let anchor = clamp_anchor(area, anchor);
    let width = size.width.min(area.width).max(1);
    let height = size.height.min(area.height);
    let x = anchor.0.min(area.right().saturating_sub(width)).max(area.x);
    let below = area.bottom().saturating_sub(anchor.1.saturating_add(1));
    let above = anchor.1.saturating_sub(area.y);
    let (y, height) = if height <= below {
        (anchor.1.saturating_add(1), height)
    } else if height <= above {
        (anchor.1.saturating_sub(height), height)
    } else if below >= above {
        (anchor.1.saturating_add(1), below)
    } else {
        (anchor.1.saturating_sub(above), above)
    };
    Rect::new(x, y, width, height)
}

fn clamp_anchor(area: Rect, anchor: (usize, usize)) -> (usize, usize) {
    (
        anchor.0.min(area.right().saturating_sub(1)).max(area.x),
        anchor.1.min(area.bottom().saturating_sub(1)).max(area.y),
    )
}

fn place_docs(input: PlacementInput, list: Rect, size: Size) -> (Option<Rect>, Option<DocsSide>) {
    for side in docs_side_order(input.preferred_docs_side) {
        let rect = match side {
            DocsSide::Right => place_right(input.area, list, size),
            DocsSide::Left => place_left(input.area, list, size),
            DocsSide::Below => place_below(input.area, list, size),
        };
        if let Some(rect) = rect {
            return (Some(rect), Some(side));
        }
    }
    (None, input.preferred_docs_side)
}

fn docs_side_order(preferred: Option<DocsSide>) -> [DocsSide; 3] {
    match preferred {
        Some(DocsSide::Right) => [DocsSide::Right, DocsSide::Left, DocsSide::Below],
        Some(DocsSide::Left) => [DocsSide::Left, DocsSide::Right, DocsSide::Below],
        Some(DocsSide::Below) => [DocsSide::Below, DocsSide::Right, DocsSide::Left],
        None => [DocsSide::Right, DocsSide::Left, DocsSide::Below],
    }
}

fn place_right(area: Rect, list: Rect, size: Size) -> Option<Rect> {
    let x = list.right().saturating_add(GAP);
    let available = area.right().saturating_sub(x);
    let width = size.width.min(available);
    if width < MIN_DOCS_WIDTH {
        return None;
    }
    let rect = side_rect(area, list, x, width, size.height);
    (rect.height > 0).then_some(rect)
}

fn place_left(area: Rect, list: Rect, size: Size) -> Option<Rect> {
    let width = size
        .width
        .min(list.x.saturating_sub(area.x).saturating_sub(GAP));
    if width < MIN_DOCS_WIDTH {
        return None;
    }
    let x = list.x.saturating_sub(GAP).saturating_sub(width);
    let rect = side_rect(area, list, x, width, size.height);
    (rect.height > 0).then_some(rect)
}

fn side_rect(area: Rect, list: Rect, x: usize, width: usize, height: usize) -> Rect {
    let available = area.bottom().saturating_sub(list.y);
    Rect::new(x, list.y, width, height.min(available))
}

fn place_below(area: Rect, list: Rect, size: Size) -> Option<Rect> {
    let y = list.bottom().saturating_add(GAP);
    let height = size.height.min(area.bottom().saturating_sub(y));
    if height == 0 {
        return None;
    }
    let width = size.width.max(list.width).min(area.width);
    let x = list.x.min(area.right().saturating_sub(width)).max(area.x);
    Some(Rect::new(x, y, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(list: Size, docs: Option<Size>) -> PlacementInput {
        PlacementInput {
            area: Rect::new(0, 0, 100, 30),
            anchor: (10, 10),
            list,
            docs,
            preferred_docs_side: None,
        }
    }

    #[test]
    fn list_opens_below_when_it_fits() {
        let placement = place(input(Size::new(24, 5), None));
        assert_eq!(placement.list, Rect::new(10, 11, 24, 5));
    }

    #[test]
    fn list_flips_above_near_the_bottom() {
        let mut value = input(Size::new(24, 5), None);
        value.anchor = (10, 27);
        let placement = place(value);
        assert_eq!(placement.list, Rect::new(10, 22, 24, 5));
    }

    #[test]
    fn list_truncates_on_the_side_with_more_space() {
        let mut value = input(Size::new(24, 20), None);
        value.anchor = (10, 10);
        let placement = place(value);
        assert_eq!(placement.list.y, 11);
        assert_eq!(placement.list.height, 19);
    }

    #[test]
    fn list_truncates_above_when_that_side_is_larger() {
        let mut value = input(Size::new(24, 20), None);
        value.anchor = (10, 20);
        let placement = place(value);
        assert_eq!(placement.list.y, 0);
        assert_eq!(placement.list.height, 20);
    }

    #[test]
    fn list_stays_inside_the_area() {
        let mut value = input(Size::new(24, 5), None);
        value.anchor = (95, 10);
        let placement = place(value);
        assert_eq!(placement.list.right(), 100);
    }

    #[test]
    fn anchor_is_clamped_to_the_area() {
        let mut value = input(Size::new(24, 5), None);
        value.anchor = (200, 200);
        let placement = place(value);
        assert_eq!(placement.list.right(), 100);
        assert_eq!(placement.list.bottom(), 29);
    }

    #[test]
    fn docs_prefer_the_right_side() {
        let placement = place(input(Size::new(24, 5), Some(Size::new(30, 10))));
        assert_eq!(placement.docs_side, Some(DocsSide::Right));
        assert_eq!(placement.docs, Some(Rect::new(35, 11, 30, 10)));
    }

    #[test]
    fn docs_keep_their_content_height() {
        let placement = place(input(Size::new(24, 12), Some(Size::new(30, 4))));
        assert_eq!(placement.docs.map(|rect| rect.height), Some(4));
    }

    #[test]
    fn docs_flip_left_near_the_right_edge() {
        let mut value = input(Size::new(24, 5), Some(Size::new(30, 10)));
        value.anchor = (90, 10);
        let placement = place(value);
        assert_eq!(placement.docs_side, Some(DocsSide::Left));
        assert!(placement.docs.expect("docs").right() < placement.list.x);
    }

    #[test]
    fn docs_fall_below_when_no_side_fits() {
        let mut value = input(Size::new(60, 5), Some(Size::new(30, 6)));
        value.area = Rect::new(0, 0, 70, 30);
        let placement = place(value);
        assert_eq!(placement.docs_side, Some(DocsSide::Below));
        assert!(placement.docs.expect("docs").y > placement.list.bottom());
    }

    #[test]
    fn preferred_side_is_tried_first() {
        let mut value = input(Size::new(24, 5), Some(Size::new(30, 10)));
        value.anchor = (30, 10);
        value.preferred_docs_side = Some(DocsSide::Left);
        let placement = place(value);
        assert_eq!(placement.docs_side, Some(DocsSide::Left));
    }

    #[test]
    fn docs_hide_when_there_is_no_room() {
        let mut value = input(Size::new(90, 5), Some(Size::new(30, 6)));
        value.area = Rect::new(0, 0, 100, 12);
        value.anchor = (45, 11);
        let placement = place(value);
        assert!(placement.docs.is_none());
    }

    #[test]
    fn docs_do_not_move_the_list() {
        let without = place(input(Size::new(24, 5), None));
        let with = place(input(Size::new(24, 5), Some(Size::new(30, 10))));
        assert_eq!(with.list, without.list);
    }
}
