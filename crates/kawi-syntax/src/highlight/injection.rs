use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::sync::Arc;

use ropey::Rope;
use tree_sitter::{Parser, QueryCursor, StreamingIterator, Tree};

use crate::language::LanguageId;
use crate::limits::MAX_CAPTURES;

use super::paint::Capture;
use super::runtime::{Runtime, RuntimeCache};

mod combined;
mod query;
mod text;

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

    pub(crate) fn captures(&mut self, request: CollectRequest<'_>) -> Option<Vec<Capture>> {
        let mut context = Context {
            text: request.text,
            visible: request.visible,
            runtimes: request.runtimes,
            parsers: &mut self.parsers,
            captures: Vec::new(),
            pending: VecDeque::new(),
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
    text: &'a Rope,
    visible: Range<usize>,
    runtimes: &'a mut RuntimeCache,
    parsers: &'a mut HashMap<LanguageId, Parser>,
    captures: Vec<Capture>,
    pending: VecDeque<Job>,
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

impl Context<'_> {
    fn collect(&mut self, layer: &Layer<'_>) -> Option<()> {
        self.collect_highlights(layer)?;
        self.enqueue(layer);
        Some(())
    }

    fn drain(&mut self) -> Option<()> {
        while let Some(job) = self.pending.pop_front() {
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
            self.collect(&layer)?;
        }
        Some(())
    }

    fn collect_highlights(&mut self, layer: &Layer<'_>) -> Option<()> {
        for block in layer.blocks {
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
