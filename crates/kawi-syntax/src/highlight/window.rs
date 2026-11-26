use std::ops::Range;

use ropey::Rope;

use crate::limits::WINDOW_CONTEXT_LINES;
use crate::text;

pub(super) fn bounds(text: &Rope, top: usize, end: usize) -> Range<usize> {
    let start = top.saturating_sub(WINDOW_CONTEXT_LINES);
    let end = end
        .saturating_add(WINDOW_CONTEXT_LINES)
        .min(text::line_count(text));
    start..end
}
