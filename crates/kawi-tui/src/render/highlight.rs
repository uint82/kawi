pub(super) mod paint;
pub(super) mod spans;

pub(super) use paint::{draw_highlight, paint_span_at, patch_span_at, write_segment};
pub(super) use spans::span_segment_at;
pub use spans::{syntax_highlight_spans, HighlightSpan, SpanView, Tint};
