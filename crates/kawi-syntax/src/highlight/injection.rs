use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::sync::Arc;

use ropey::Rope;
use tree_sitter::{Parser, QueryCursor, StreamingIterator, Tree};

use crate::language::LanguageId;
use crate::limits::MAX_CAPTURES;

use super::locals::{self, LocalSource, Resolution};
use super::paint::Capture;
use super::runtime::{Runtime, RuntimeCache};

mod combined;
mod query;
pub(super) mod text;

const MAX_INJECTION_DEPTH: usize = 3;

pub(crate) struct CollectRequest<'a> {
    pub(crate) text: &'a Rope,
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

    pub(crate) fn captures(&mut self, request: CollectRequest<'_>) -> Vec<Capture> {
        let mut context = Context {
            text: request.text,
            visible: request.visible,
            runtimes: request.runtimes,
            parsers: &mut self.parsers,
            captures: Vec::new(),
            pending: VecDeque::new(),
            limit_reached: false,
        };
        let source = 0..request.text.len_bytes();
        let host = Layer {
            text: request.text,
            text_source: source.clone(),
            tree: request.tree,
            runtime: request.runtime,
            source,
            blocks: request.blocks,
            layer: 0,
            language: request.language,
        };
        context.collect(&host);
        context.drain();
        context
            .captures
            .sort_by_key(|capture| (capture.priority, capture.layer, capture.start));
        context.captures
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

struct Context<'a> {
    text: &'a Rope,
    visible: Range<usize>,
    runtimes: &'a mut RuntimeCache,
    parsers: &'a mut HashMap<LanguageId, Parser>,
    captures: Vec<Capture>,
    pending: VecDeque<Job>,
    limit_reached: bool,
}

struct Layer<'a> {
    text: &'a Rope,
    text_source: Range<usize>,
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
    text_source: Range<usize>,
    text: Option<Rope>,
    layer: usize,
    language: LanguageId,
}

fn resolve_references(captures: &mut [Capture], locals: &Resolution) {
    let definitions = captures
        .iter()
        .filter(|capture| locals.definitions().contains(&(capture.start..capture.end)))
        .map(|capture| (capture.start..capture.end, capture.highlight))
        .collect::<HashMap<_, _>>();
    for capture in captures {
        let range = capture.start..capture.end;
        if let Some(definition) = locals.definition_for(&range) {
            if let Some(highlight) = definitions.get(definition) {
                capture.highlight = *highlight;
            }
        }
    }
}

impl Context<'_> {
    fn collect(&mut self, layer: &Layer<'_>) {
        let locals = self.local_resolution(layer);
        self.collect_highlights(layer, &locals);
        self.enqueue(layer);
    }

    fn local_resolution(&self, layer: &Layer<'_>) -> Resolution {
        let Some(query) = layer.runtime.locals.as_ref() else {
            return Resolution::default();
        };
        locals::resolve(
            query,
            LocalSource {
                tree: layer.tree,
                text: layer.text,
                text_source: layer.text_source.clone(),
                source: layer.source.clone(),
            },
        )
    }

    fn drain(&mut self) {
        while !self.limit_reached {
            let Some(job) = self.pending.pop_front() else {
                break;
            };
            let text = job.text.clone().unwrap_or_else(|| self.text.clone());
            let block = 0..job.source.len();
            let layer = Layer {
                text: &text,
                text_source: job.text_source.clone(),
                tree: &job.tree,
                runtime: &job.runtime,
                source: job.source.clone(),
                blocks: std::slice::from_ref(&block),
                layer: job.layer,
                language: job.language,
            };
            self.collect(&layer);
        }
    }

    fn collect_highlights(&mut self, layer: &Layer<'_>, locals: &Resolution) {
        let mut captures = Vec::new();
        'blocks: for block in layer.blocks {
            let mut cursor = QueryCursor::new();
            cursor.set_byte_range(block.clone());
            let mut matches = cursor.captures(
                &layer.runtime.config.query,
                layer.tree.root_node(),
                text::NodeText {
                    text: layer.text,
                    source: layer.text_source.clone(),
                },
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
                let start = layer.source.start + capture.node.start_byte();
                let end = layer.source.start + capture.node.end_byte();
                if layer
                    .runtime
                    .non_local
                    .get(matched.pattern_index)
                    .copied()
                    .unwrap_or(false)
                    && locals.is_local(&(start..end))
                {
                    continue;
                }
                if self.captures.len() + captures.len() >= MAX_CAPTURES {
                    self.limit_reached = true;
                    break 'blocks;
                }
                captures.push(Capture {
                    start,
                    end,
                    highlight,
                    layer: layer.layer,
                    priority: layer
                        .runtime
                        .priorities
                        .get(matched.pattern_index)
                        .copied()
                        .unwrap_or(0),
                });
            }
        }
        resolve_references(&mut captures, locals);
        self.captures.extend(captures);
    }

    fn enqueue(&mut self, parent: &Layer<'_>) {
        if self.limit_reached || parent.layer >= MAX_INJECTION_DEPTH {
            return;
        }
        for injection in query::injections(parent) {
            if injection.language == parent.language {
                continue;
            }
            let Some(prepared) = combined::prepare(&injection, self.text, &self.visible) else {
                continue;
            };
            let Some(runtime) = self.runtimes.get(injection.language) else {
                continue;
            };
            let (parse_text, parse_range) = match prepared.text.as_ref() {
                Some(masked) => (masked, 0..masked.len_bytes()),
                None => (self.text, prepared.source.clone()),
            };
            let Some(tree) =
                text::parse_embedded(self.parsers, parse_text, injection.language, &parse_range)
            else {
                continue;
            };
            let text_source = match prepared.text.as_ref() {
                Some(masked) => 0..masked.len_bytes(),
                None => prepared.source.clone(),
            };
            self.pending.push_back(Job {
                tree,
                runtime,
                source: prepared.source,
                text_source,
                text: prepared.text,
                layer: parent.layer + 1,
                language: injection.language,
            });
        }
    }
}
