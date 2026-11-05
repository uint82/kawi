use std::ops::Range;

use super::util::{find_byte, find_ignore_ascii_case, merge_ranges, skip_whitespace};
use super::value;

const STYLE_OPEN: &[u8] = b"<style";
const STYLE_CLOSE: &[u8] = b"</style";
const STYLE_ATTRIBUTE: &[u8] = b"style";
const TAG_END: u8 = b'>';
const EQUALS: u8 = b'=';
const QUOTE_SINGLE: u8 = b'\'';
const QUOTE_DOUBLE: u8 = b'"';
const SLASH: u8 = b'/';

#[derive(Default)]
pub(crate) struct MarkupState {
    pub(super) in_style: bool,
}

pub(super) fn ranges(line: &str, state: &mut MarkupState) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    for block in style_block_ranges(line, state) {
        add_value_ranges(line, block, false, &mut ranges);
    }
    for attribute in style_attribute_ranges(line) {
        add_value_ranges(line, attribute, true, &mut ranges);
    }
    merge_ranges(ranges)
}

fn add_value_ranges(
    line: &str,
    range: Range<usize>,
    commit_at_end: bool,
    out: &mut Vec<Range<usize>>,
) {
    let Some(content) = line.get(range.clone()) else {
        return;
    };
    for part in value::ranges(content, commit_at_end) {
        out.push(range.start.saturating_add(part.start)..range.start.saturating_add(part.end));
    }
}

pub(super) fn style_attribute_ranges(line: &str) -> Vec<Range<usize>> {
    let bytes = line.as_bytes();
    let mut ranges = Vec::new();
    let mut cursor = 0;
    while let Some(index) = find_ignore_ascii_case(bytes, STYLE_ATTRIBUTE, cursor) {
        let name_end = index.saturating_add(STYLE_ATTRIBUTE.len());
        if inside_tag(bytes, index) && is_attribute_boundary(bytes, index, name_end) {
            if let Some((range, next)) = attribute_value_range(bytes, name_end) {
                ranges.push(range);
                cursor = next;
                continue;
            }
        }
        cursor = index.saturating_add(1);
    }
    ranges
}

fn inside_tag(bytes: &[u8], index: usize) -> bool {
    let before = bytes.get(..index).unwrap_or_default();
    let open = before.iter().rposition(|byte| *byte == b'<');
    let close = before.iter().rposition(|byte| *byte == b'>');
    open.is_some_and(|open| close.is_none_or(|close| close < open))
}

fn is_attribute_boundary(bytes: &[u8], start: usize, end: usize) -> bool {
    let before = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index))
        .is_none_or(|byte| !is_attribute_name_byte(*byte));
    let after = bytes
        .get(end)
        .is_none_or(|byte| byte.is_ascii_whitespace() || *byte == EQUALS);
    before && after
}

fn is_attribute_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b':'
}

fn attribute_value_range(bytes: &[u8], after_name: usize) -> Option<(Range<usize>, usize)> {
    let equals = skip_whitespace(bytes, after_name);
    if bytes.get(equals) != Some(&EQUALS) {
        return None;
    }
    let start = skip_whitespace(bytes, equals.saturating_add(1));
    match bytes.get(start).copied() {
        Some(quote @ (QUOTE_SINGLE | QUOTE_DOUBLE)) => {
            let content_start = start.saturating_add(1);
            let end = find_byte(bytes, content_start, quote).unwrap_or(bytes.len());
            let next = end.saturating_add(1).min(bytes.len());
            Some((content_start..end, next))
        }
        Some(_) => {
            let end = find_attribute_end(bytes, start);
            Some((start..end, end))
        }
        None => None,
    }
}

fn find_attribute_end(bytes: &[u8], start: usize) -> usize {
    bytes
        .get(start..)
        .and_then(|tail| {
            tail.iter()
                .position(|byte| byte.is_ascii_whitespace() || *byte == TAG_END)
        })
        .map_or(bytes.len(), |offset| start.saturating_add(offset))
}

pub(super) fn style_block_ranges(line: &str, state: &mut MarkupState) -> Vec<Range<usize>> {
    let bytes = line.as_bytes();
    let mut ranges = Vec::new();
    let mut cursor = 0;
    if state.in_style {
        let Some(close) = find_ignore_ascii_case(bytes, STYLE_CLOSE, 0) else {
            push_range(&mut ranges, 0..bytes.len());
            return ranges;
        };
        push_range(&mut ranges, 0..close);
        state.in_style = false;
        cursor = after_tag(bytes, close);
    }
    while let Some(open) = find_style_open(bytes, cursor) {
        let content_start = after_tag(bytes, open);
        if let Some(close) = find_ignore_ascii_case(bytes, STYLE_CLOSE, content_start) {
            push_range(&mut ranges, content_start..close);
            cursor = after_tag(bytes, close);
        } else {
            push_range(&mut ranges, content_start..bytes.len());
            state.in_style = true;
            break;
        }
    }
    ranges
}

fn push_range(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    if range.start < range.end {
        ranges.push(range);
    }
}

fn find_style_open(bytes: &[u8], from: usize) -> Option<usize> {
    let mut cursor = from;
    loop {
        let index = find_ignore_ascii_case(bytes, STYLE_OPEN, cursor)?;
        let after = index.saturating_add(STYLE_OPEN.len());
        if bytes
            .get(after)
            .is_some_and(|byte| byte.is_ascii_whitespace() || matches!(*byte, TAG_END | SLASH))
        {
            return Some(index);
        }
        cursor = index.saturating_add(1);
    }
}

fn after_tag(bytes: &[u8], start: usize) -> usize {
    find_byte(bytes, start, TAG_END).map_or(bytes.len(), |index| index.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::super::preview_ranges;
    use super::{style_attribute_ranges, style_block_ranges, MarkupState};
    use crate::app::color_preview::mode::PreviewMode;

    fn contains(ranges: &[std::ops::Range<usize>], index: usize) -> bool {
        ranges
            .iter()
            .any(|range| range.start <= index && index < range.end)
    }

    #[test]
    fn extracts_quoted_and_unquoted_style_attributes() {
        let line = r#"<div style="color: red" style='background: blue' style=other>"#;
        let ranges = style_attribute_ranges(line);
        assert_eq!(ranges.len(), 3);
        assert_eq!(&line[ranges[0].clone()], "color: red");
        assert_eq!(&line[ranges[1].clone()], "background: blue");
        assert_eq!(&line[ranges[2].clone()], "other");
    }

    #[test]
    fn ignores_style_substrings_in_attribute_names() {
        let line = r#"<div data-style="color: red">"#;
        assert!(style_attribute_ranges(line).is_empty());
    }

    #[test]
    fn ignores_style_attributes_outside_tags() {
        let line = r#"<p>style="color: red"</p>"#;
        assert!(style_attribute_ranges(line).is_empty());
    }

    #[test]
    fn tracks_style_blocks_across_lines() {
        let mut state = MarkupState::default();
        assert!(style_block_ranges("<style>", &mut state).is_empty());
        assert!(state.in_style);
        let line = ".x { color: red; }";
        let ranges = style_block_ranges(line, &mut state);
        assert_eq!(&line[ranges[0].clone()], line);
        assert!(style_block_ranges("</style><p>red</p>", &mut state).is_empty());
        assert!(!state.in_style);
    }

    #[test]
    fn restricts_named_colors_in_markup() {
        let line = r#"<div style="color: red">red</div>"#;
        let mut state = MarkupState::default();
        let ranges = preview_ranges(line, PreviewMode::Markup, &mut state);
        let style_value = line.find("red\"").expect("style value");
        let text = line.rfind("red").expect("text");
        assert!(contains(&ranges, style_value));
        assert!(!contains(&ranges, text));
    }
}
