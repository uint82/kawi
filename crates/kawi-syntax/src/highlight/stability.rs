use std::collections::HashSet;
use std::ops::Range;

use ropey::Rope;

use crate::registry::{registry, SyntaxHighlight};
use crate::remap::remap_span;
use crate::rows::RowWindow;
use crate::span::{SyntaxEdit, SyntaxSpan};

use super::paint;

pub(crate) struct StableHighlights {
    text: Rope,
    spans: Vec<SyntaxSpan>,
}

#[derive(Clone, Copy)]
pub(crate) struct StableWindow<'a> {
    pub(crate) rows: &'a Range<usize>,
    pub(crate) errors: &'a HashSet<usize>,
}

impl StableHighlights {
    pub(crate) fn new(text: &Rope, spans: Vec<SyntaxSpan>) -> Self {
        Self {
            text: text.clone(),
            spans,
        }
    }

    pub(crate) fn spans(
        &self,
        text: &Rope,
        edits: &[SyntaxEdit],
        target: StableWindow<'_>,
    ) -> Vec<SyntaxSpan> {
        self.spans
            .iter()
            .filter_map(|span| remap_span(*span, &self.text, text, edits))
            .filter(|span| target.rows.contains(&span.row) && target.errors.contains(&span.row))
            .collect()
    }
}

pub(crate) fn overlay(
    text: &Rope,
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
