pub(super) mod paint;
pub(super) mod patch;
pub(super) mod spans;

pub(super) use paint::draw_highlight;
pub(super) use patch::{apply_patch_at, patch_spans_at, PatchTarget};
pub(super) use spans::span_segment_at;
pub use spans::{syntax_highlight_spans, HighlightSpan, SpanView, Tint};
