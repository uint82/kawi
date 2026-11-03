use std::ops::Range;

use tree_sitter::{Parser, QueryCursor, Range as TreeRange, StreamingIterator, Tree};

use crate::diff::{input_edit, LineIndex};
use crate::language::LanguageId;
use crate::limits::{MAX_CAPTURES, WINDOW_CONTEXT_LINES, WINDOW_PARSE_BYTES};
use crate::rows::{self, RowWindow};
use crate::span::{SyntaxRequest, SyntaxSpan};

mod paint;
mod runtime;

pub(crate) use runtime::Runtime;

pub(crate) struct Document {
    parser: Parser,
    text: String,
    tree: Option<Tree>,
    revision: u64,
    index: LineIndex,
    window_rows: Range<usize>,
    large: bool,
}

impl Document {
    pub(crate) fn new(language: LanguageId) -> Self {
        let mut parser = Parser::new();
        let _ = parser.set_language(&language.language());
        Self {
            parser,
            text: String::new(),
            tree: None,
            revision: 0,
            index: LineIndex::new(""),
            window_rows: 0..0,
            large: false,
        }
    }

    pub(crate) fn update(
        &mut self,
        request: &SyntaxRequest,
        runtime: &Runtime,
    ) -> (Range<usize>, Vec<SyntaxSpan>) {
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
        let window = if self.large {
            self.window_rows.clone()
        } else {
            window_bounds(&self.index, request.key.top, request.key.end)
        };
        let spans = self.spans(runtime, window.clone());
        (window, spans)
    }

    fn refresh_text(&mut self, request: &SyntaxRequest) {
        self.text = request.text.to_string();
        self.index = LineIndex::new(&self.text);
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
        self.tree = self.parser.parse(&self.text, None);
        self.large = true;
        self.window_rows = window;
        self.revision = request.key.revision;
    }

    fn window_covers(&self, request: &SyntaxRequest) -> bool {
        self.window_rows.start <= request.key.top && request.key.end <= self.window_rows.end
    }

    fn spans(&self, runtime: &Runtime, window: Range<usize>) -> Vec<SyntaxSpan> {
        let Some(tree) = self.tree.as_ref() else {
            return Vec::new();
        };
        let rows = rows::visible_rows(&self.text, &self.index, window.start, window.end);
        if rows.is_empty() {
            return Vec::new();
        }
        let Some(captures) = self.captures(tree, runtime, &rows) else {
            return Vec::new();
        };
        paint::paint_captures(&self.text, &rows, &captures)
    }

    fn captures(
        &self,
        tree: &Tree,
        runtime: &Runtime,
        rows: &[RowWindow],
    ) -> Option<Vec<paint::Capture>> {
        let mut captures = Vec::new();
        for block in rows::query_blocks(rows) {
            let mut cursor = QueryCursor::new();
            cursor.set_byte_range(block);
            let mut matches = cursor.captures(
                &runtime.config.query,
                tree.root_node(),
                self.text.as_bytes(),
            );
            while let Some((matched, capture_index)) = matches.next() {
                let Some(capture) = matched.captures().get(*capture_index) else {
                    continue;
                };
                let Some(highlight) = runtime
                    .captures
                    .get(capture.index as usize)
                    .copied()
                    .flatten()
                else {
                    continue;
                };
                if captures.len() >= MAX_CAPTURES {
                    return None;
                }
                captures.push(paint::Capture {
                    start: capture.node.start_byte(),
                    end: capture.node.end_byte(),
                    highlight,
                });
            }
        }
        captures.sort_by_key(|capture| capture.start);
        Some(captures)
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
