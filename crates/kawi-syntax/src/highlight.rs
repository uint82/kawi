use std::ops::Range;

use tree_sitter::{Parser, Range as TreeRange, Tree};

use crate::diff::{input_edit, LineIndex};
use crate::language::LanguageId;
use crate::limits::{WINDOW_CONTEXT_LINES, WINDOW_PARSE_BYTES};
use crate::rows;
use crate::span::{SyntaxRequest, SyntaxSpan};

mod errors;
mod injection;
mod paint;
mod runtime;
mod stability;

pub(crate) use runtime::{Runtime, RuntimeCache};

use stability::StableHighlights;

pub(crate) struct Document {
    parser: Parser,
    language: LanguageId,
    text: String,
    tree: Option<Tree>,
    revision: u64,
    index: LineIndex,
    window_rows: Range<usize>,
    large: bool,
    collector: injection::Collector,
    embedded_runtimes: RuntimeCache,
    stable: Option<StableHighlights>,
}

impl Document {
    pub(crate) fn new(language: LanguageId) -> Self {
        let mut parser = Parser::new();
        let _ = parser.set_language(&language.language());
        Self {
            parser,
            language,
            text: String::new(),
            tree: None,
            revision: 0,
            index: LineIndex::new(""),
            window_rows: 0..0,
            large: false,
            collector: injection::Collector::new(),
            embedded_runtimes: RuntimeCache::new(),
            stable: None,
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
            window_bounds(&self.index, request.key.top, request.key.end)
        };
        let (spans, has_error) = self.spans(runtime, window.clone());
        if !has_error && self.tree.is_some() {
            self.refresh_stable(window.clone(), &spans);
        }
        (window, spans)
    }

    fn ensure_tree(&mut self, request: &SyntaxRequest) {
        let large = request.text.len_bytes() > WINDOW_PARSE_BYTES;
        let revision_changed = request.key.revision != self.revision;
        if large {
            if revision_changed || self.tree.is_none() || !self.large {
                self.refresh_text(request);
            }
            if revision_changed
                || !self.large
                || self.tree.is_none()
                || !self.window_covers(request)
            {
                self.parse_window(request);
            }
        } else if self.tree.is_none() || revision_changed || self.large {
            self.reparse_full(request);
        }
    }

    fn refresh_stable(&mut self, window: Range<usize>, spans: &[SyntaxSpan]) {
        self.stable = if self.large {
            StableHighlights::window(&self.text, &self.index, window, spans.to_vec())
        } else {
            Some(StableHighlights::full(
                &self.text,
                &self.index,
                spans.to_vec(),
            ))
        };
    }

    fn refresh_text(&mut self, request: &SyntaxRequest) {
        let text = request.text.to_string();
        let index = LineIndex::new(&text);
        if let Some(mut tree) = self.tree.take() {
            if let Some(edit) = input_edit(&self.text, &text, &self.index, &index) {
                tree.edit(&edit);
            }
            self.tree = Some(tree);
        }
        self.text = text;
        self.index = index;
        self.revision = request.key.revision;
    }

    fn reparse_full(&mut self, request: &SyntaxRequest) {
        let text = request.text.to_string();
        let index = LineIndex::new(&text);
        let _ = self.parser.set_included_ranges(&[]);
        let old_tree = if self.large {
            self.tree.take();
            None
        } else {
            self.tree.take()
        };
        let tree = match old_tree {
            Some(mut tree) => match input_edit(&self.text, &text, &self.index, &index) {
                Some(edit) => {
                    tree.edit(&edit);
                    self.parser.parse(&text, Some(&tree))
                }
                None => Some(tree),
            },
            None => self.parser.parse(&text, None),
        };
        self.text = text;
        self.index = index;
        self.revision = request.key.revision;
        self.tree = tree;
        self.large = false;
        self.window_rows = 0..self.index.line_count();
    }

    fn parse_window(&mut self, request: &SyntaxRequest) {
        let window = window_bounds(&self.index, request.key.top, request.key.end);
        let start = window.start;
        let end = window.end;
        let start_byte = self.index.line_start(start);
        let end_byte = if end < self.index.line_count() {
            self.index.line_start(end)
        } else {
            self.text.len()
        };
        let range = TreeRange {
            start_byte,
            end_byte,
            start_point: self.index.point(start_byte),
            end_point: self.index.point(end_byte),
        };
        let _ = self.parser.set_included_ranges(&[range]);
        let old_tree = self.tree.take();
        self.tree = self.parser.parse(&self.text, old_tree.as_ref());
        self.large = true;
        self.window_rows = window;
        self.revision = request.key.revision;
    }

    fn window_covers(&self, request: &SyntaxRequest) -> bool {
        self.window_rows.start <= request.key.top && request.key.end <= self.window_rows.end
    }

    fn spans(&mut self, runtime: &Runtime, window: Range<usize>) -> (Vec<SyntaxSpan>, bool) {
        let Some(tree) = self.tree.as_ref() else {
            return (Vec::new(), false);
        };
        let has_error = tree.root_node().has_error();
        let rows = rows::visible_rows(&self.text, &self.index, window.start, window.end);
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
            let error_rows = errors::error_rows(tree, &self.index, &window);
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
        let stable_spans = stable.spans(&self.text, &self.index, target);
        stability::overlay(&self.text, rows, spans, stable_spans)
    }
}

fn window_bounds(index: &LineIndex, top: usize, end: usize) -> Range<usize> {
    let start = top.saturating_sub(WINDOW_CONTEXT_LINES);
    let end = end
        .saturating_add(WINDOW_CONTEXT_LINES)
        .min(index.line_count());
    start..end
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
