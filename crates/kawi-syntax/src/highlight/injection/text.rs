use std::collections::HashMap;
use std::ops::Range;

use ropey::iter::Chunks;
use ropey::Rope;
use tree_sitter::{Node, Parser, Query, QueryMatch, TextProvider, Tree};

use crate::language::LanguageId;

use super::super::parse::parse_slice;

pub(super) struct NodeText<'a> {
    pub(super) text: &'a Rope,
    pub(super) source: Range<usize>,
}

impl<'a> TextProvider<&'a str> for NodeText<'a> {
    type I = Chunks<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let start = self
            .source
            .start
            .saturating_add(node.start_byte())
            .min(self.text.len_bytes());
        let end = self
            .source
            .start
            .saturating_add(node.end_byte())
            .min(self.text.len_bytes())
            .max(start);
        self.text
            .get_byte_slice(start..end)
            .map(|slice| slice.chunks())
            .unwrap_or_else(|| self.text.byte_slice(0..0).chunks())
    }
}

pub(super) fn injection_language(
    query: &Query,
    matched: &QueryMatch<'_, '_>,
    text: &Rope,
    source: &Range<usize>,
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
    let start = source.start + capture.node.start_byte();
    let end = source.start + capture.node.end_byte();
    let name = crate::text::slice(text, start..end)?;
    LanguageId::from_alias(&name)
}

pub(super) fn parse_embedded(
    parsers: &mut HashMap<LanguageId, Parser>,
    text: &Rope,
    language: LanguageId,
    range: &Range<usize>,
) -> Option<Tree> {
    let source = text.get_byte_slice(range.clone())?;
    let parser = parsers.entry(language).or_insert_with(|| {
        let mut parser = Parser::new();
        let _ = parser.set_language(&language.language());
        parser
    });
    parse_slice(parser, source, None)
}

pub(super) fn intersect(range: &Range<usize>, visible: &Range<usize>) -> Range<usize> {
    range.start.max(visible.start)..range.end.min(visible.end)
}
