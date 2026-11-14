use std::collections::{HashMap, HashSet};
use std::ops::Range;

use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

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
            let includes_children = includes_children(query, matched.pattern_index);
            for capture in matched.captures() {
                if capture.index != content_index {
                    continue;
                }
                for range in content_ranges(capture.node, includes_children, layer.source.start) {
                    if range.is_empty()
                        || !seen.insert((range.start, range.end, language, matched.pattern_index))
                    {
                        continue;
                    }
                    if is_combined {
                        combined
                            .entry((language, matched.pattern_index))
                            .or_default()
                            .push(range);
                    } else {
                        single.push(Injection {
                            language,
                            ranges: std::iter::once(range).collect(),
                            combined: false,
                        });
                    }
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

fn includes_children(query: &Query, pattern_index: usize) -> bool {
    query
        .property_settings(pattern_index)
        .iter()
        .any(|property| property.key.as_ref() == "injection.include-children")
}

fn content_ranges(
    node: Node<'_>,
    includes_children: bool,
    source_start: usize,
) -> Vec<Range<usize>> {
    if includes_children {
        return std::iter::once(source_start + node.start_byte()..source_start + node.end_byte())
            .collect();
    }
    let mut ranges = Vec::new();
    let mut cursor = node.walk();
    let mut start = node.start_byte();
    for child in node.children(&mut cursor) {
        if child.start_byte() > start {
            ranges.push(source_start + start..source_start + child.start_byte());
        }
        start = start.max(child.end_byte());
    }
    if start < node.end_byte() {
        ranges.push(source_start + start..source_start + node.end_byte());
    }
    ranges
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

#[cfg(test)]
mod tests {
    use tree_sitter::{Parser, Tree};

    use super::*;
    use crate::language::LanguageId;

    fn rust_block() -> (Tree, Range<usize>) {
        let mut parser = Parser::new();
        parser
            .set_language(&LanguageId::Rust.language())
            .expect("language");
        let tree = parser
            .parse("fn main() { let value = 1; }\n", None)
            .expect("tree");
        let block = tree
            .root_node()
            .child(0)
            .and_then(|item| item.child_by_field_name("body"))
            .expect("block");
        let range = block.start_byte()..block.end_byte();
        (tree, range)
    }

    #[test]
    fn excludes_child_ranges_by_default() {
        let (tree, block_range) = rust_block();
        let block = tree
            .root_node()
            .descendant_for_byte_range(block_range.start, block_range.end)
            .expect("block");
        let ranges = content_ranges(block, false, 0);
        assert!(!ranges
            .iter()
            .any(|range| range.contains(&(block_range.start + 3))));
    }

    #[test]
    fn include_children_returns_the_whole_node() {
        let (tree, block_range) = rust_block();
        let block = tree
            .root_node()
            .descendant_for_byte_range(block_range.start, block_range.end)
            .expect("block");
        assert_eq!(
            content_ranges(block, true, 0),
            vec![block.start_byte()..block.end_byte()]
        );
    }
}
