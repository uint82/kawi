use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::sync::Arc;

use tree_sitter::{Parser, Query, QueryCursor, QueryMatch, StreamingIterator, Tree};

use crate::language::LanguageId;
use crate::limits::MAX_CAPTURES;

use super::paint::Capture;
use super::runtime::{Runtime, RuntimeCache};

const MAX_INJECTION_DEPTH: usize = 3;

pub(crate) struct CollectRequest<'a> {
    pub(crate) text: &'a str,
    pub(crate) tree: &'a Tree,
    pub(crate) runtime: &'a Runtime,
    pub(crate) language: LanguageId,
    pub(crate) blocks: &'a [Range<usize>],
    pub(crate) visible: Range<usize>,
    pub(crate) runtimes: &'a mut RuntimeCache,
}

pub(crate) struct Collector {
    parsers: HashMap<LanguageId, Parser>,
}

impl Collector {
    pub(crate) fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }

    pub(crate) fn captures(&mut self, request: CollectRequest<'_>) -> Option<Vec<Capture>> {
        let mut context = Context {
            text: request.text,
            visible: request.visible,
            runtimes: request.runtimes,
            parsers: &mut self.parsers,
            captures: Vec::new(),
            pending: VecDeque::new(),
        };
        let host = Layer {
            tree: request.tree,
            runtime: request.runtime,
            source: 0..request.text.len(),
            blocks: request.blocks,
            layer: 0,
            language: request.language,
        };
        context.collect(&host)?;
        context.drain()?;
        context
            .captures
            .sort_by_key(|capture| (capture.layer, capture.start));
        Some(context.captures)
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

struct Context<'a> {
    text: &'a str,
    visible: Range<usize>,
    runtimes: &'a mut RuntimeCache,
    parsers: &'a mut HashMap<LanguageId, Parser>,
    captures: Vec<Capture>,
    pending: VecDeque<Job>,
}

struct Layer<'a> {
    tree: &'a Tree,
    runtime: &'a Runtime,
    source: Range<usize>,
    blocks: &'a [Range<usize>],
    layer: usize,
    language: LanguageId,
}

struct Job {
    tree: Tree,
    runtime: Arc<Runtime>,
    source: Range<usize>,
    layer: usize,
    language: LanguageId,
}

struct Injection {
    language: LanguageId,
    range: Range<usize>,
}

impl Context<'_> {
    fn collect(&mut self, layer: &Layer<'_>) -> Option<()> {
        self.collect_highlights(layer)?;
        self.enqueue(layer);
        Some(())
    }

    fn drain(&mut self) -> Option<()> {
        while let Some(job) = self.pending.pop_front() {
            let block = 0..job.source.len();
            let layer = Layer {
                tree: &job.tree,
                runtime: &job.runtime,
                source: job.source.clone(),
                blocks: std::slice::from_ref(&block),
                layer: job.layer,
                language: job.language,
            };
            self.collect(&layer)?;
        }
        Some(())
    }

    fn collect_highlights(&mut self, layer: &Layer<'_>) -> Option<()> {
        let source = self.text.get(layer.source.clone())?;
        for block in layer.blocks {
            let mut cursor = QueryCursor::new();
            cursor.set_byte_range(block.clone());
            let mut matches = cursor.captures(
                &layer.runtime.config.query,
                layer.tree.root_node(),
                source.as_bytes(),
            );
            while let Some((matched, capture_index)) = matches.next() {
                let Some(capture) = matched.captures().get(*capture_index) else {
                    continue;
                };
                let Some(highlight) = layer
                    .runtime
                    .captures
                    .get(capture.index as usize)
                    .copied()
                    .flatten()
                else {
                    continue;
                };
                if self.captures.len() >= MAX_CAPTURES {
                    return None;
                }
                self.captures.push(Capture {
                    start: layer.source.start + capture.node.start_byte(),
                    end: layer.source.start + capture.node.end_byte(),
                    highlight,
                    layer: layer.layer,
                });
            }
        }
        Some(())
    }

    fn enqueue(&mut self, parent: &Layer<'_>) {
        if parent.layer >= MAX_INJECTION_DEPTH {
            return;
        }
        for injection in self.injections(parent) {
            if injection.language == parent.language {
                continue;
            }
            let range = intersect(&injection.range, &self.visible);
            if range.is_empty() {
                continue;
            }
            let Some(runtime) = self.runtimes.get(injection.language) else {
                continue;
            };
            let Some(tree) = parse_embedded(self.parsers, self.text, injection.language, &range)
            else {
                continue;
            };
            self.pending.push_back(Job {
                tree,
                runtime,
                source: range,
                layer: parent.layer + 1,
                language: injection.language,
            });
        }
    }

    fn injections(&self, layer: &Layer<'_>) -> Vec<Injection> {
        let Some(query) = layer.runtime.injections.as_ref() else {
            return Vec::new();
        };
        let Some(source) = self.text.get(layer.source.clone()) else {
            return Vec::new();
        };
        let Some(content_index) = query.capture_index_for_name("injection.content") else {
            return Vec::new();
        };
        let language_index = query.capture_index_for_name("injection.language");
        let mut injections = HashSet::new();
        for block in layer.blocks {
            let mut cursor = QueryCursor::new();
            cursor.set_byte_range(block.clone());
            let mut matches = cursor.matches(query, layer.tree.root_node(), source.as_bytes());
            while let Some(matched) = matches.next() {
                let Some(language) = injection_language(query, matched, source, language_index)
                else {
                    continue;
                };
                for capture in matched.captures() {
                    if capture.index != content_index {
                        continue;
                    }
                    let start = layer.source.start + capture.node.start_byte();
                    let end = layer.source.start + capture.node.end_byte();
                    if start < end {
                        injections.insert((start, end, language));
                    }
                }
            }
        }
        let mut injections = injections
            .into_iter()
            .map(|(start, end, language)| Injection {
                language,
                range: start..end,
            })
            .collect::<Vec<_>>();
        injections.sort_by_key(|injection| (injection.range.start, injection.range.end));
        injections
    }
}

fn injection_language(
    query: &Query,
    matched: &QueryMatch<'_, '_>,
    source: &str,
    capture_index: Option<u32>,
) -> Option<LanguageId> {
    let name = query
        .property_settings(matched.pattern_index)
        .iter()
        .find(|property| property.key.as_ref() == "injection.language")
        .and_then(|property| property.value.as_deref());
    if let Some(name) = name {
        return LanguageId::from_alias(name);
    }
    let capture_index = capture_index?;
    let capture = matched
        .captures()
        .iter()
        .find(|capture| capture.index == capture_index)?;
    let name = capture.node.utf8_text(source.as_bytes()).ok()?;
    LanguageId::from_alias(name)
}

fn parse_embedded(
    parsers: &mut HashMap<LanguageId, Parser>,
    text: &str,
    language: LanguageId,
    range: &Range<usize>,
) -> Option<Tree> {
    let source = text.get(range.clone())?;
    let parser = parsers.entry(language).or_insert_with(|| {
        let mut parser = Parser::new();
        let _ = parser.set_language(&language.language());
        parser
    });
    parser.parse(source, None)
}

fn intersect(range: &Range<usize>, visible: &Range<usize>) -> Range<usize> {
    range.start.max(visible.start)..range.end.min(visible.end)
}
