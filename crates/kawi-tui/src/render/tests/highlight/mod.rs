use super::*;
use crate::frame::SearchSpan;
use crate::render::bracket::bracket_match_spans;
use crate::render::highlight::paint::{hidden_cell_offset, paint_span_at};
use crate::render::highlight::spans::{preview_highlight_spans, search_highlight_spans};
use crate::render::highlight::syntax_highlight_spans;
use kawi_syntax::SyntaxSpan;

mod paint;
mod spans;
