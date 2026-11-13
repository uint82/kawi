use std::ops::Range;

use ropey::Rope;
use tree_sitter::{InputEdit, Parser, Range as TreeRange, Tree};

use crate::diff::diff_edit;
use crate::language::LanguageId;
use crate::limits::{MAX_STABLE_EDITS, WINDOW_CONTEXT_LINES, WINDOW_PARSE_BYTES};
use crate::rows;
use crate::span::{SyntaxEdit, SyntaxRequest, SyntaxSpan};
use crate::text;

mod errors;
mod injection;
mod paint;
mod parse;
mod runtime;
mod stability;

pub(crate) use runtime::{Runtime, RuntimeCache};

use stability::StableHighlights;

pub(crate) struct Document {
    parser: Parser,
    language: LanguageId,
    text: Rope,
    tree: Option<Tree>,
    revision: u64,
    window_rows: Range<usize>,
    large: bool,
    collector: injection::Collector,
    embedded_runtimes: RuntimeCache,
    stable: Option<StableHighlights>,
    stable_edits: Vec<SyntaxEdit>,
    initialized: bool,
}

impl Document {
    pub(crate) fn new(language: LanguageId) -> Self {
        let mut parser = Parser::new();
        let _ = parser.set_language(&language.language());
        Self {
            parser,
            language,
            text: Rope::new(),
            tree: None,
            revision: 0,
            window_rows: 0..0,
            large: false,
            collector: injection::Collector::new(),
            embedded_runtimes: RuntimeCache::new(),
            stable: None,
            stable_edits: Vec::new(),
            initialized: false,
        }
    }

    pub(crate) fn update(
        &mut self,
        request: &SyntaxRequest,
        runtime: &Runtime,
    ) -> (Range<usize>, Vec<SyntaxSpan>) {
        self.ensure_tree(request);
        let window = if self.large {
            self.window_rows.clone()
        } else {
            window_bounds(&self.text, request.key.top, request.key.end)
        };
        let (spans, has_error) = self.spans(runtime, window.clone());
        if !has_error && self.tree.is_some() {
            self.refresh_stable(spans.clone());
        }
        (window, spans)
    }

    fn ensure_tree(&mut self, request: &SyntaxRequest) {
        let large = request.text.len_bytes() > WINDOW_PARSE_BYTES;
        let revision_changed = request.key.revision != self.revision;
        let was_large = self.large;
        if revision_changed || self.tree.is_none() || large != self.large {
            self.refresh_text(request);
        }
        if large {
            if revision_changed
                || self.tree.is_none()
                || !self.large
                || !self.window_covers(request)
            {
                self.parse_window(request);
            }
        } else {
            if was_large {
                self.tree = None;
            }
            if self.tree.is_none() || revision_changed || was_large {
                self.reparse_full();
            }
        }
    }

    fn refresh_text(&mut self, request: &SyntaxRequest) {
        if self.apply_edits(request) {
            self.text = request.text.clone();
            self.revision = request.key.revision;
            self.initialized = true;
            return;
        }
        if self.initialized && self.tree.is_some() {
            if request.edits.is_empty() {
                if let Some(edit) = diff_edit(&self.text, &request.text) {
                    if self.apply_edit(&edit) {
                        self.track_stable_edit(&edit);
                    } else {
                        self.reset_stable();
                    }
                }
            } else {
                self.tree = None;
                self.reset_stable();
            }
        }
        self.text = request.text.clone();
        self.revision = request.key.revision;
        self.initialized = true;
    }

    fn apply_edits(&mut self, request: &SyntaxRequest) -> bool {
        if !self.initialized || request.base_revision != self.revision || request.edits.is_empty() {
            return false;
        }
        for edit in &request.edits {
            if !self.apply_edit(edit) {
                return false;
            }
            self.track_stable_edit(edit);
        }
        true
    }

    fn track_stable_edit(&mut self, edit: &SyntaxEdit) {
        if self.stable.is_none() {
            return;
        }
        if self.stable_edits.len() >= MAX_STABLE_EDITS {
            self.reset_stable();
            return;
        }
        self.stable_edits.push(edit.clone());
    }

    fn apply_edit(&mut self, edit: &SyntaxEdit) -> bool {
        let length = self.text.len_chars();
        if edit.start > edit.end || edit.end > length {
            return false;
        }
        let start_byte = self.text.char_to_byte(edit.start);
        let old_end_byte = self.text.char_to_byte(edit.end);
        let start_position = text::point(&self.text, start_byte);
        let old_end_position = text::point(&self.text, old_end_byte);
        self.text.remove(edit.start..edit.end);
        self.text.insert(edit.start, &edit.text);
        let inserted = edit.text.chars().count();
        let new_end = edit
            .start
            .saturating_add(inserted)
            .min(self.text.len_chars());
        let new_end_byte = self.text.char_to_byte(new_end);
        let new_end_position = text::point(&self.text, new_end_byte);
        if let Some(tree) = self.tree.as_mut() {
            tree.edit(&InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position,
                old_end_position,
                new_end_position,
            });
        }
        true
    }

    fn reset_stable(&mut self) {
        self.stable = None;
        self.stable_edits.clear();
    }

    fn refresh_stable(&mut self, spans: Vec<SyntaxSpan>) {
        self.stable = Some(StableHighlights::new(&self.text, spans));
        self.stable_edits.clear();
    }

    fn reparse_full(&mut self) {
        let _ = self.parser.set_included_ranges(&[]);
        let old_tree = self.tree.take();
        self.tree = parse::parse_rope(&mut self.parser, &self.text, old_tree.as_ref());
        self.large = false;
        self.window_rows = 0..text::line_count(&self.text);
    }

    fn parse_window(&mut self, request: &SyntaxRequest) {
        let window = window_bounds(&self.text, request.key.top, request.key.end);
        let range = TreeRange {
            start_byte: text::line_start(&self.text, window.start),
            end_byte: text::line_start(&self.text, window.end),
            start_point: text::point(&self.text, text::line_start(&self.text, window.start)),
            end_point: text::point(&self.text, text::line_start(&self.text, window.end)),
        };
        let _ = self.parser.set_included_ranges(&[range]);
        let old_tree = self.tree.take();
        self.tree = parse::parse_rope(&mut self.parser, &self.text, old_tree.as_ref());
        self.large = true;
        self.window_rows = window;
    }

    fn window_covers(&self, request: &SyntaxRequest) -> bool {
        self.window_rows.start <= request.key.top && request.key.end <= self.window_rows.end
    }

    fn spans(&mut self, runtime: &Runtime, window: Range<usize>) -> (Vec<SyntaxSpan>, bool) {
        let Some(tree) = self.tree.as_ref() else {
            return (Vec::new(), false);
        };
        let has_error = tree.root_node().has_error();
        let rows = rows::visible_rows(&self.text, window.start, window.end);
        let (Some(first), Some(last)) = (rows.first(), rows.last()) else {
            return (Vec::new(), has_error);
        };
        let visible = first.start..last.text_end;
        let blocks = rows::query_blocks(&rows);
        let Some(captures) = self.collector.captures(injection::CollectRequest {
            text: &self.text,
            tree,
            runtime,
            language: self.language,
            blocks: &blocks,
            visible,
            runtimes: &mut self.embedded_runtimes,
        }) else {
            return (Vec::new(), has_error);
        };
        let mut spans = paint::paint_captures(&self.text, &rows, &captures);
        if has_error {
            let error_rows = errors::error_rows(tree, &self.text, &window);
            let target = stability::StableWindow {
                rows: &window,
                errors: &error_rows,
            };
            spans = self.overlay_stable(&rows, target, spans);
        }
        (spans, has_error)
    }

    fn overlay_stable(
        &self,
        rows: &[rows::RowWindow],
        target: stability::StableWindow<'_>,
        spans: Vec<SyntaxSpan>,
    ) -> Vec<SyntaxSpan> {
        let Some(stable) = self.stable.as_ref() else {
            return spans;
        };
        let stable_spans = stable.spans(&self.text, &self.stable_edits, target);
        stability::overlay(&self.text, rows, spans, stable_spans)
    }
}

fn window_bounds(text: &Rope, top: usize, end: usize) -> Range<usize> {
    let start = top.saturating_sub(WINDOW_CONTEXT_LINES);
    let end = end
        .saturating_add(WINDOW_CONTEXT_LINES)
        .min(text::line_count(text));
    start..end
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
