use std::collections::{HashMap, HashSet};
use std::ops::Range;

use ropey::Rope;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

use crate::text;

use super::injection::text::NodeText;

#[derive(Default)]
pub(super) struct Resolution {
    definitions: HashSet<Range<usize>>,
    references: HashMap<Range<usize>, Range<usize>>,
}

impl Resolution {
    pub(super) fn definitions(&self) -> &HashSet<Range<usize>> {
        &self.definitions
    }

    pub(super) fn is_local(&self, range: &Range<usize>) -> bool {
        self.definitions.contains(range) || self.references.contains_key(range)
    }

    pub(super) fn definition_for(&self, range: &Range<usize>) -> Option<&Range<usize>> {
        self.references.get(range)
    }
}

pub(super) struct LocalSource<'a> {
    pub(super) tree: &'a tree_sitter::Tree,
    pub(super) text: &'a Rope,
    pub(super) text_source: Range<usize>,
    pub(super) source: Range<usize>,
}

struct Scope {
    range: Range<usize>,
    inherits: bool,
    definitions: Vec<Definition>,
}

struct Definition {
    name: String,
    range: Range<usize>,
    value_end: usize,
}

pub(super) fn resolve(query: &Query, source: LocalSource<'_>) -> Resolution {
    let Some(definition_index) = query.capture_index_for_name("local.definition") else {
        return Resolution::default();
    };
    let reference_index = query.capture_index_for_name("local.reference");
    let scope_index = query.capture_index_for_name("local.scope");
    let value_index = query.capture_index_for_name("local.definition-value");
    let mut resolution = Resolution::default();
    let mut scopes = vec![Scope {
        range: 0..usize::MAX,
        inherits: false,
        definitions: Vec::new(),
    }];
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(
        query,
        source.tree.root_node(),
        NodeText {
            text: source.text,
            source: source.text_source.clone(),
        },
    );
    while let Some(matched) = matches.next() {
        let value_end = matched
            .captures()
            .iter()
            .find(|capture| Some(capture.index) == value_index)
            .map_or(0, |capture| absolute(source.source.start, capture.node).end);
        for capture in matched.captures() {
            let range = absolute(source.source.start, capture.node);
            pop_finished(&mut scopes, range.start);
            if Some(capture.index) == scope_index {
                scopes.push(Scope {
                    range,
                    inherits: scope_inherits(query, matched.pattern_index),
                    definitions: Vec::new(),
                });
            } else if capture.index == definition_index {
                let Some(name) = capture_name(source.text, source.source.start, capture.node)
                else {
                    continue;
                };
                if let Some(scope) = scopes.last_mut() {
                    scope.definitions.push(Definition {
                        name,
                        range: range.clone(),
                        value_end,
                    });
                }
                resolution.definitions.insert(range);
            } else if Some(capture.index) == reference_index {
                let Some(name) = capture_name(source.text, source.source.start, capture.node)
                else {
                    continue;
                };
                if let Some(definition) = resolve_reference(&scopes, &range, &name) {
                    resolution.references.insert(range, definition);
                }
            }
        }
    }
    resolution
}

fn pop_finished(scopes: &mut Vec<Scope>, position: usize) {
    while scopes
        .last()
        .is_some_and(|scope| position >= scope.range.end)
    {
        scopes.pop();
    }
}

fn resolve_reference(scopes: &[Scope], range: &Range<usize>, name: &str) -> Option<Range<usize>> {
    for scope in scopes.iter().rev() {
        if let Some(definition) = scope
            .definitions
            .iter()
            .rev()
            .find(|definition| definition.name == name && range.start >= definition.value_end)
        {
            return Some(definition.range.clone());
        }
        if !scope.inherits {
            return None;
        }
    }
    None
}

fn capture_name(text: &Rope, source_start: usize, node: tree_sitter::Node<'_>) -> Option<String> {
    let start = source_start + node.start_byte();
    let end = source_start + node.end_byte();
    text::slice(text, start..end)
}

fn scope_inherits(query: &Query, pattern: usize) -> bool {
    query
        .property_settings(pattern)
        .iter()
        .find(|property| property.key.as_ref() == "local.scope-inherits")
        .and_then(|property| property.value.as_deref())
        .is_none_or(|value| value == "true")
}

fn absolute(source_start: usize, node: tree_sitter::Node<'_>) -> Range<usize> {
    source_start + node.start_byte()..source_start + node.end_byte()
}
