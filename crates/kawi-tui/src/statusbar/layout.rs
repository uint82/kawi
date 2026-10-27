use super::prune;
use super::segment::{Section, Segment, SegmentId, SegmentStyle};
use super::text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedSegment {
    pub id: Option<SegmentId>,
    pub x: usize,
    pub width: usize,
    pub text: String,
    pub style: SegmentStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub spans: Vec<PlacedSegment>,
    pub width: usize,
}

impl StatusLine {
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

pub fn layout(segments: &[Segment], width: usize, separator: &str) -> StatusLine {
    if width == 0 {
        return StatusLine {
            spans: Vec::new(),
            width: 0,
        };
    }
    let separator = text::sanitize(separator);
    let separator_width = text::display_width(&separator);
    let (mut left, mut center, mut right) = split_sections(segments);
    if !center_fits(&left, &center, &right, width, separator_width) {
        center.clear();
    }
    prune::drop_until_fits(&mut left, &mut right, width, separator_width);
    if total_width(&left, &right, separator_width) > width {
        prune::truncate_file(&mut left, &mut right, width, separator_width);
    }
    if total_width(&left, &right, separator_width) > width {
        prune::shorten_mode(&mut left, &mut center, &mut right);
    }
    if total_width(&left, &right, separator_width) > width {
        prune::remove_file(&mut left, &mut center, &mut right);
    }
    if total_width(&left, &right, separator_width) > width {
        prune::keep_position(&mut left, &mut center, &mut right);
    }
    place(&left, &center, &right, width, separator_width, &separator)
}

fn split_sections(segments: &[Segment]) -> (Vec<Segment>, Vec<Segment>, Vec<Segment>) {
    let mut left = Vec::new();
    let mut center = Vec::new();
    let mut right = Vec::new();
    for segment in segments {
        let text = text::sanitize(&segment.text);
        if text.is_empty() {
            continue;
        }
        let segment = Segment {
            text,
            ..segment.clone()
        };
        match segment.section {
            Section::Left => left.push(segment),
            Section::Center => center.push(segment),
            Section::Right => right.push(segment),
        }
    }
    (left, center, right)
}

pub(super) fn total_width(left: &[Segment], right: &[Segment], separator_width: usize) -> usize {
    section_width(left, separator_width).saturating_add(section_width(right, separator_width))
}

pub(super) fn section_width(segments: &[Segment], separator_width: usize) -> usize {
    let mut width: usize = 0;
    let mut count: usize = 0;
    for segment in segments {
        if segment.text.is_empty() {
            continue;
        }
        width = width.saturating_add(text::display_width(&segment.text));
        count += 1;
    }
    width.saturating_add(separator_width.saturating_mul(count.saturating_sub(1)))
}

fn center_fits(
    left: &[Segment],
    center: &[Segment],
    right: &[Segment],
    width: usize,
    separator_width: usize,
) -> bool {
    let center_width = section_width(center, separator_width);
    if center_width == 0 {
        return false;
    }
    let left_width = section_width(left, separator_width);
    let right_width = section_width(right, separator_width);
    if left_width
        .saturating_add(center_width)
        .saturating_add(right_width)
        > width
    {
        return false;
    }
    let start = width.saturating_sub(center_width) / 2;
    let end = start.saturating_add(center_width);
    let left_clear = left_width == 0 || start >= left_width.saturating_add(separator_width);
    let right_clear = right_width == 0
        || end.saturating_add(separator_width) <= width.saturating_sub(right_width);
    left_clear && right_clear
}

fn place(
    left: &[Segment],
    center: &[Segment],
    right: &[Segment],
    width: usize,
    separator_width: usize,
    separator: &str,
) -> StatusLine {
    let mut spans = Vec::new();
    place_section(left, 0, width, separator, &mut spans);
    let right_width = section_width(right, separator_width);
    let right_start = width.saturating_sub(right_width);
    place_section(
        right,
        right_start,
        width.saturating_sub(right_start),
        separator,
        &mut spans,
    );
    if center_fits(left, center, right, width, separator_width) {
        let center_width = section_width(center, separator_width);
        let center_start = width.saturating_sub(center_width) / 2;
        place_section(center, center_start, center_width, separator, &mut spans);
    }
    spans.sort_by_key(|span| span.x);
    StatusLine { spans, width }
}

fn place_section(
    segments: &[Segment],
    start: usize,
    max_width: usize,
    separator: &str,
    spans: &mut Vec<PlacedSegment>,
) {
    let separator_width = text::display_width(separator);
    let mut x = start;
    let mut first = true;
    let limit = start.saturating_add(max_width);
    for segment in segments {
        if segment.text.is_empty() {
            continue;
        }
        if !first {
            if x.saturating_add(separator_width) >= limit {
                break;
            }
            if separator_width > 0 {
                spans.push(PlacedSegment {
                    id: None,
                    x,
                    width: separator_width,
                    text: separator.to_string(),
                    style: SegmentStyle::Default,
                });
            }
            x = x.saturating_add(separator_width);
        }
        let remaining = limit.saturating_sub(x);
        if remaining == 0 {
            break;
        }
        let text = text::truncate_right(&segment.text, remaining);
        let width = text::display_width(&text);
        if width == 0 {
            continue;
        }
        spans.push(PlacedSegment {
            id: Some(segment.id),
            x,
            width,
            text,
            style: segment.style,
        });
        x = x.saturating_add(width);
        first = false;
    }
}
