use super::*;
use crate::frame::{DiagnosticSpan, DiagnosticUnderline};
use crate::render::diagnostic::{
    diagnostic_highlight_spans, diagnostic_segments, paint_diagnostic, DiagnosticPaint,
};

mod paint;
mod segments;
