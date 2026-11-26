use super::*;
use crate::frame::{DiagnosticSeverity, DiagnosticSpan, DiagnosticTag, DiagnosticUnderline};
use crate::render::diagnostic::{
    diagnostic_highlight_spans, patch_decorations, patch_tag_foreground, DiagnosticLayer,
};

mod overlay;
mod spans;
