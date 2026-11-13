use std::collections::{HashMap, HashSet};
use std::ops::Range;

use tree_sitter::{Query, QueryCursor, StreamingIterator};

use super::text::{injection_language, NodeText};
use super::Layer;
use crate::language::LanguageId;

pub(super) struct Injection {
    pub(super) language: LanguageId,
    pub(super) ranges: Vec<Range<usize>>,
    pub(super) combined: bool,
}

pub(super) fn injections(layer: &Layer<'_>) -> Vec<Injection> {
    let Some(query) = layer.runtime.injections.as_ref() else {
        return Vec::new();
    };
    let Some(content_index) = query.capture_index_for_name("injection.content") else {
        return Vec::new();
    };
    let language_index = query.capture_index_for_name("injection.language");
    let mut single = Vec::new();
    let mut combined: HashMap<(LanguageId, usize), Vec<Range<usize>>> = HashMap::new();
    let mut seen = HashSet::new();
    for block in layer.blocks {
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(block.clone());
        let mut matches = cursor.matches(
            query,
            layer.tree.root_node(),
            NodeText {
                text: layer.text,
                source: layer.text_source.clone(),
            },
        );
        while let Some(matched) = matches.next() {
            let Some(language) = injection_language(
                query,
                matched,
                layer.text,
                &layer.text_source,
                language_index,
            ) else {
                continue;
            };
            let is_combined = is_combined(query, matched.pattern_index);
            for capture in matched.captures() {
                if capture.index != content_index {
                    continue;
                }
                let start = layer.source.start + capture.node.start_byte();
                let end = layer.source.start + capture.node.end_byte();
                if start >= end || !seen.insert((start, end, language, matched.pattern_index)) {
                    continue;
                }
                if is_combined {
                    combined
                        .entry((language, matched.pattern_index))
                        .or_default()
                        .push(start..end);
                } else {
                    single.push(Injection {
                        language,
                        ranges: std::iter::once(start..end).collect(),
                        combined: false,
                    });
                }
            }
        }
    }
    single.extend(combined.into_iter().map(|((language, _), mut ranges)| {
        merge_ranges(&mut ranges);
        Injection {
            language,
            ranges,
            combined: true,
        }
    }));
    single.sort_by_key(|injection| {
        injection
            .ranges
            .first()
            .map_or(usize::MAX, |range| range.start)
    });
    single
}

fn is_combined(query: &Query, pattern_index: usize) -> bool {
    query
        .property_settings(pattern_index)
        .iter()
        .any(|property| property.key.as_ref() == "injection.combined")
}

fn merge_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges.drain(..) {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    *ranges = merged;
}
