use super::layout::total_width;
use super::segment::{Section, Segment, SegmentId};
use super::text;

pub(super) fn drop_until_fits(
    left: &mut Vec<Segment>,
    right: &mut Vec<Segment>,
    width: usize,
    separator_width: usize,
) {
    while total_width(left, right, separator_width) > width {
        let Some((section, index)) = next_drop(left, right) else {
            break;
        };
        match section {
            Section::Left => {
                left.remove(index);
            }
            Section::Right => {
                right.remove(index);
            }
            Section::Center => {}
        }
    }
}

fn next_drop(left: &[Segment], right: &[Segment]) -> Option<(Section, usize)> {
    let mut candidates = Vec::new();
    for (index, segment) in left.iter().enumerate() {
        if segment.is_required() || segment.flex {
            continue;
        }
        candidates.push((segment.priority, 1u8, index, Section::Left));
    }
    for (index, segment) in right.iter().enumerate() {
        if segment.is_required() || segment.flex {
            continue;
        }
        candidates.push((segment.priority, 0u8, index, Section::Right));
    }
    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(left.1.cmp(&right.1))
            .then(right.2.cmp(&left.2))
    });
    candidates
        .first()
        .map(|candidate| (candidate.3, candidate.2))
}

pub(super) fn truncate_file(
    left: &mut [Segment],
    right: &mut [Segment],
    width: usize,
    separator_width: usize,
) {
    let excess = total_width(left, right, separator_width).saturating_sub(width);
    let Some(segment) = find_file_mut(left, right) else {
        return;
    };
    let current = text::display_width(&segment.text);
    let target = current
        .saturating_sub(excess)
        .max(segment.min_width)
        .min(width);
    if target < current {
        segment.text = text::truncate_left(&segment.text, target);
    }
}

fn find_file_mut<'a>(left: &'a mut [Segment], right: &'a mut [Segment]) -> Option<&'a mut Segment> {
    if let Some(index) = left
        .iter()
        .position(|segment| segment.id == SegmentId::File)
    {
        return left.get_mut(index);
    }
    let index = right
        .iter()
        .position(|segment| segment.id == SegmentId::File)?;
    right.get_mut(index)
}

pub(super) fn remove_padding(left: &mut [Segment], center: &mut [Segment], right: &mut [Segment]) {
    for segment in left
        .iter_mut()
        .chain(center.iter_mut())
        .chain(right.iter_mut())
    {
        segment.padding = 0;
    }
}

pub(super) fn shorten_mode(left: &mut [Segment], center: &mut [Segment], right: &mut [Segment]) {
    if let Some(segment) = find_mut(left, center, right, SegmentId::Mode) {
        segment.text = segment
            .text
            .chars()
            .next()
            .map(String::from)
            .unwrap_or_default();
    }
}

pub(super) fn remove_file(
    left: &mut Vec<Segment>,
    center: &mut Vec<Segment>,
    right: &mut Vec<Segment>,
) {
    left.retain(|segment| segment.id != SegmentId::File);
    center.retain(|segment| segment.id != SegmentId::File);
    right.retain(|segment| segment.id != SegmentId::File);
}

pub(super) fn keep_position(
    left: &mut Vec<Segment>,
    center: &mut Vec<Segment>,
    right: &mut Vec<Segment>,
) {
    let position = left
        .iter()
        .chain(center.iter())
        .chain(right.iter())
        .find(|segment| segment.id == SegmentId::Position)
        .cloned();
    left.clear();
    center.clear();
    right.clear();
    if let Some(position) = position {
        right.push(position);
    }
}

fn find_mut<'a>(
    left: &'a mut [Segment],
    center: &'a mut [Segment],
    right: &'a mut [Segment],
    id: SegmentId,
) -> Option<&'a mut Segment> {
    if let Some(index) = left.iter().position(|segment| segment.id == id) {
        return left.get_mut(index);
    }
    if let Some(index) = center.iter().position(|segment| segment.id == id) {
        return center.get_mut(index);
    }
    let index = right.iter().position(|segment| segment.id == id)?;
    right.get_mut(index)
}
