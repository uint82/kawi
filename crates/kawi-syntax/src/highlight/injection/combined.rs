use std::ops::Range;

use ropey::Rope;

use super::query::Injection;
use super::text::intersect;

pub(super) struct Prepared {
    pub(super) source: Range<usize>,
    pub(super) text: Option<Rope>,
}

pub(super) fn prepare(
    injection: &Injection,
    text: &Rope,
    visible: &Range<usize>,
) -> Option<Prepared> {
    if !injection
        .ranges
        .iter()
        .any(|range| !intersect(range, visible).is_empty())
    {
        return None;
    }
    if !injection.combined {
        let range = injection.ranges.first()?;
        return Some(Prepared {
            source: intersect(range, visible),
            text: None,
        });
    }
    let source = injection.ranges.first()?.start..injection.ranges.last()?.end;
    Some(Prepared {
        source: source.clone(),
        text: Some(mask(text, &injection.ranges, &source)),
    })
}

fn mask(text: &Rope, ranges: &[Range<usize>], source: &Range<usize>) -> Rope {
    let mut masked = String::with_capacity(source.len());
    let mut cursor = source.start;
    for range in ranges {
        let start = range.start.max(source.start);
        let end = range.end.min(source.end);
        if start > cursor {
            mask_bytes(text, cursor..start, &mut masked);
        }
        if start < end {
            masked.push_str(&text.byte_slice(start..end).to_string());
        }
        cursor = cursor.max(end);
    }
    if cursor < source.end {
        mask_bytes(text, cursor..source.end, &mut masked);
    }
    Rope::from_str(&masked)
}

fn mask_bytes(text: &Rope, range: Range<usize>, masked: &mut String) {
    for byte in text.byte_slice(range).bytes() {
        masked.push(if matches!(byte, b'\n' | b'\r') {
            byte as char
        } else {
            ' '
        });
    }
}
