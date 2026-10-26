use crate::frame::Rect;

const WIDTH_PERCENT: usize = 85;
const HEIGHT_PERCENT: usize = 70;
const MIN_WIDTH: usize = 40;
const MIN_HEIGHT: usize = 8;
const MAX_WIDTH: usize = 120;
const MAX_HEIGHT: usize = 30;
const SIDE_BY_SIDE_MIN_WIDTH: usize = 64;
const SIDE_BY_SIDE_MIN_HEIGHT: usize = 5;
const STACKED_MIN_WIDTH: usize = 24;
const STACKED_MIN_HEIGHT: usize = 8;
const MIN_RESULTS_WIDTH: usize = 20;
const MIN_PREVIEW_WIDTH: usize = 20;
const MIN_RESULTS_HEIGHT: usize = 3;
const MIN_PREVIEW_HEIGHT: usize = 3;
const RESULTS_PERCENT: usize = 50;
const STACKED_RESULTS_PERCENT: usize = 55;
const PROMPT_HEIGHT: usize = 1;
const RULE_HEIGHT: usize = 1;
const FOOTER_HEIGHT: usize = 1;
const BORDER_WIDTH: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickerLayout {
    pub frame: Rect,
    pub prompt: Rect,
    pub rule: Rect,
    pub results: Rect,
    pub divider: Option<Rect>,
    pub preview: Option<Rect>,
    pub footer: Rect,
}

pub fn layout(area: Rect, has_preview: bool) -> PickerLayout {
    let frame = centered_frame(area);
    let inner = inner_rect(frame);
    let prompt = Rect::new(inner.x, inner.y, inner.width, PROMPT_HEIGHT);
    let rule = Rect::new(inner.x, prompt.bottom(), inner.width, RULE_HEIGHT);
    let footer = Rect::new(
        inner.x,
        inner.bottom().saturating_sub(FOOTER_HEIGHT),
        inner.width,
        FOOTER_HEIGHT.min(inner.height),
    );
    let body = Rect::new(
        inner.x,
        rule.bottom(),
        inner.width,
        footer.y.saturating_sub(rule.bottom()),
    );
    let mut layout = PickerLayout {
        frame,
        prompt,
        rule,
        results: body,
        divider: None,
        preview: None,
        footer,
    };
    if has_preview {
        apply_preview(&mut layout, body);
    }
    layout
}

fn centered_frame(area: Rect) -> Rect {
    let width = preferred_size(area.width, WIDTH_PERCENT, MIN_WIDTH, MAX_WIDTH);
    let height = preferred_size(area.height, HEIGHT_PERCENT, MIN_HEIGHT, MAX_HEIGHT);
    Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width) / 2),
        area.y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width,
        height,
    )
}

fn preferred_size(available: usize, percent: usize, minimum: usize, maximum: usize) -> usize {
    let preferred = available.saturating_mul(percent) / 100;
    let lower = minimum.min(available);
    let upper = maximum.min(available).max(lower);
    preferred.clamp(lower, upper)
}

fn inner_rect(frame: Rect) -> Rect {
    Rect::new(
        frame.x.saturating_add(1),
        frame.y.saturating_add(1),
        frame.width.saturating_sub(BORDER_WIDTH),
        frame.height.saturating_sub(BORDER_WIDTH),
    )
}

fn apply_preview(layout: &mut PickerLayout, body: Rect) {
    if let Some((results, divider, preview)) = side_by_side(body) {
        layout.results = results;
        layout.divider = Some(divider);
        layout.preview = Some(preview);
        return;
    }
    if let Some((results, divider, preview)) = stacked(body) {
        layout.results = results;
        layout.divider = Some(divider);
        layout.preview = Some(preview);
    }
}

fn side_by_side(body: Rect) -> Option<(Rect, Rect, Rect)> {
    if body.width < SIDE_BY_SIDE_MIN_WIDTH || body.height < SIDE_BY_SIDE_MIN_HEIGHT {
        return None;
    }
    let results_width = side_by_side_results_width(body.width)?;
    let results = Rect::new(body.x, body.y, results_width, body.height);
    let divider = Rect::new(results.right(), body.y, 1, body.height);
    let preview = Rect::new(
        divider.right(),
        body.y,
        body.right().saturating_sub(divider.right()),
        body.height,
    );
    Some((results, divider, preview))
}

fn side_by_side_results_width(total: usize) -> Option<usize> {
    let maximum = total.saturating_sub(MIN_PREVIEW_WIDTH.saturating_add(1));
    let width = total.saturating_mul(RESULTS_PERCENT) / 100;
    (width >= MIN_RESULTS_WIDTH && maximum >= MIN_RESULTS_WIDTH).then(|| width.min(maximum))
}

fn stacked(body: Rect) -> Option<(Rect, Rect, Rect)> {
    if body.width < STACKED_MIN_WIDTH || body.height < STACKED_MIN_HEIGHT {
        return None;
    }
    let maximum = body
        .height
        .saturating_sub(MIN_PREVIEW_HEIGHT.saturating_add(1));
    let results_height = (body.height.saturating_mul(STACKED_RESULTS_PERCENT) / 100)
        .clamp(MIN_RESULTS_HEIGHT, maximum);
    let results = Rect::new(body.x, body.y, body.width, results_height);
    let divider = Rect::new(body.x, results.bottom(), body.width, 1);
    let preview = Rect::new(
        body.x,
        divider.bottom(),
        body.width,
        body.bottom().saturating_sub(divider.bottom()),
    );
    Some((results, divider, preview))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: usize, height: usize) -> Rect {
        Rect::new(0, 0, width, height)
    }

    #[test]
    fn centers_the_frame_within_the_area() {
        let layout = layout(area(120, 40), false);
        assert_eq!(layout.frame, Rect::new(9, 6, 102, 28));
        assert_eq!(layout.prompt, Rect::new(10, 7, 100, 1));
        assert_eq!(layout.footer, Rect::new(10, 32, 100, 1));
    }

    #[test]
    fn splits_results_and_preview_side_by_side_when_wide() {
        let layout = layout(area(120, 40), true);
        assert_eq!(layout.results, Rect::new(10, 9, 50, 23));
        assert_eq!(layout.divider, Some(Rect::new(60, 9, 1, 23)));
        assert_eq!(layout.preview, Some(Rect::new(61, 9, 49, 23)));
    }

    #[test]
    fn stacks_preview_below_results_when_narrow() {
        let layout = layout(area(60, 24), true);
        assert_eq!(layout.results, Rect::new(5, 7, 49, 6));
        assert_eq!(layout.divider, Some(Rect::new(5, 13, 49, 1)));
        assert_eq!(layout.preview, Some(Rect::new(5, 14, 49, 4)));
    }

    #[test]
    fn hides_preview_when_the_body_is_too_small() {
        let layout = layout(area(30, 10), true);
        assert_eq!(layout.results, Rect::new(1, 4, 28, 3));
        assert_eq!(layout.divider, None);
        assert_eq!(layout.preview, None);
    }

    #[test]
    fn uses_the_full_body_without_preview() {
        let layout = layout(area(80, 24), false);
        assert_eq!(layout.results, Rect::new(7, 7, 66, 11));
        assert_eq!(layout.divider, None);
        assert_eq!(layout.preview, None);
    }

    #[test]
    fn keeps_the_frame_inside_small_areas() {
        let layout = layout(area(20, 6), true);
        assert_eq!(layout.frame, Rect::new(0, 0, 20, 6));
        assert_eq!(layout.divider, None);
        assert_eq!(layout.preview, None);
    }
}
