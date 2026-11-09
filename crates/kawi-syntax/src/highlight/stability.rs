use std::collections::HashSet;
use std::ops::Range;

use crate::diff::{input_edit, LineIndex};
use crate::registry::{registry, SyntaxHighlight};
use crate::rows::RowWindow;
use crate::span::SyntaxSpan;

use super::paint;

mod mapping;

use mapping::{global_span, keep_span, local_span, remap, window_text, SpanMap};

enum StableSource {
    Full {
        text: String,
        index: LineIndex,
    },
    Window {
        text: String,
        index: LineIndex,
        window: Range<usize>,
    },
}

pub(crate) struct StableHighlights {
    source: StableSource,
    spans: Vec<SyntaxSpan>,
}

#[derive(Clone, Copy)]
pub(crate) struct StableWindow<'a> {
    pub(crate) rows: &'a Range<usize>,
    pub(crate) errors: &'a HashSet<usize>,
}

impl StableHighlights {
    pub(crate) fn full(text: &str, index: &LineIndex, spans: Vec<SyntaxSpan>) -> Self {
        Self {
            source: StableSource::Full {
                text: text.to_string(),
                index: index.clone(),
            },
            spans,
        }
    }

    pub(crate) fn window(
        text: &str,
        index: &LineIndex,
        window: Range<usize>,
        spans: Vec<SyntaxSpan>,
    ) -> Option<Self> {
        let window_text = window_text(text, index, &window)?;
        let window_index = LineIndex::new(window_text);
        let spans = spans
            .into_iter()
            .filter_map(|span| local_span(span, window.start))
            .collect();
        Some(Self {
            source: StableSource::Window {
                text: window_text.to_string(),
                index: window_index,
                window,
            },
            spans,
        })
    }

    pub(crate) fn spans(
        &self,
        text: &str,
        index: &LineIndex,
        target: StableWindow<'_>,
    ) -> Vec<SyntaxSpan> {
        match &self.source {
            StableSource::Full {
                text: old_text,
                index: old_index,
            } => {
                let edit = input_edit(old_text, text, old_index, index);
                let map = SpanMap {
                    old_text,
                    old_index,
                    new_text: text,
                    new_index: index,
                    edit: edit.as_ref(),
                };
                remap(&self.spans, &map, |span| {
                    keep_span(span, target.rows, target.errors)
                })
            }
            StableSource::Window {
                text: old_text,
                index: old_index,
                window: old_window,
            } => {
                if *old_window != *target.rows {
                    return Vec::new();
                }
                let Some(new_text) = window_text(text, index, target.rows) else {
                    return Vec::new();
                };
                let new_index = LineIndex::new(new_text);
                let edit = input_edit(old_text, new_text, old_index, &new_index);
                let map = SpanMap {
                    old_text,
                    old_index,
                    new_text,
                    new_index: &new_index,
                    edit: edit.as_ref(),
                };
                remap(&self.spans, &map, |span| {
                    global_span(span, target.rows, target.errors)
                })
            }
        }
    }
}

pub(crate) fn overlay(
    text: &str,
    rows: &[RowWindow],
    current: Vec<SyntaxSpan>,
    stable: Vec<SyntaxSpan>,
) -> Vec<SyntaxSpan> {
    if stable.is_empty() {
        return current;
    }
    let mut cells = paint::blank_cells(text, rows);
    paint::paint_spans(rows, &mut cells, &current);
    paint_stable(rows, &mut cells, &stable);
    paint::merge_cells(rows, cells)
}

fn paint_stable(
    rows: &[RowWindow],
    cells: &mut [Vec<Option<SyntaxHighlight>>],
    spans: &[SyntaxSpan],
) {
    for span in spans {
        let Some(line) = paint::cells_for_span(rows, cells, *span) else {
            continue;
        };
        for cell in line {
            if should_override(span.highlight, *cell) {
                *cell = Some(span.highlight);
            }
        }
    }
}

fn should_override(stable: SyntaxHighlight, current: Option<SyntaxHighlight>) -> bool {
    let Some(current) = current else {
        return true;
    };
    if stable == current {
        return false;
    }
    let Some(stable_name) = registry().name(stable) else {
        return true;
    };
    let Some(current_name) = registry().name(current) else {
        return true;
    };
    !is_descendant(current_name, stable_name)
}

fn is_descendant(name: &str, parent: &str) -> bool {
    name.strip_prefix(parent)
        .is_some_and(|rest| rest.starts_with('.'))
}
