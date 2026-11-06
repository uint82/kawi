use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

use crate::language::LanguageId;

mod capture_names;
#[cfg(test)]
mod tests;

use capture_names::capture_names;

const BASE_NAMES: &[&str] = &[
    "attribute",
    "boolean",
    "character",
    "comment",
    "comment.documentation",
    "constant",
    "constant.builtin",
    "constant.macro",
    "constant.numeric",
    "constructor",
    "constructor.builtin",
    "decorator",
    "embedded",
    "error",
    "escape",
    "function",
    "function.builtin",
    "function.call",
    "function.macro",
    "function.method",
    "invalid",
    "keyword",
    "keyword.control",
    "keyword.directive",
    "keyword.function",
    "keyword.import",
    "keyword.operator",
    "keyword.return",
    "keyword.storage",
    "label",
    "module",
    "namespace",
    "number",
    "operator",
    "property",
    "property.builtin",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "special",
    "string",
    "string.escape",
    "string.regexp",
    "string.special",
    "string.special.key",
    "string.special.symbol",
    "tag",
    "tag.attribute",
    "tag.delimiter",
    "type",
    "type.builtin",
    "type.definition",
    "variable",
    "variable.builtin",
    "variable.member",
    "variable.parameter",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxHighlight(pub u16);

#[derive(Debug)]
pub struct HighlightRegistry {
    names: Vec<String>,
    indices: HashMap<String, SyntaxHighlight>,
}

impl HighlightRegistry {
    fn new() -> Self {
        let mut names = BASE_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect::<BTreeSet<_>>();
        for language in LanguageId::ALL {
            names.extend(
                capture_names(language.highlights())
                    .into_iter()
                    .map(str::to_string),
            );
        }
        let names = names.into_iter().collect::<Vec<_>>();
        assert!(
            names.len() < usize::from(u16::MAX),
            "syntax capture registry exceeds u16"
        );
        let indices = names
            .iter()
            .enumerate()
            .map(|(index, name)| (name.clone(), SyntaxHighlight(index as u16)))
            .collect();
        Self { names, indices }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn name(&self, highlight: SyntaxHighlight) -> Option<&str> {
        self.names.get(usize::from(highlight.0)).map(String::as_str)
    }

    pub fn index_of(&self, name: &str) -> Option<SyntaxHighlight> {
        self.indices.get(name).copied()
    }

    pub fn resolve(&self, capture: &str) -> Option<SyntaxHighlight> {
        let mut name = capture;
        loop {
            if let Some(highlight) = self.index_of(name) {
                return Some(highlight);
            }
            let (parent, _) = name.rsplit_once('.')?;
            name = parent;
        }
    }
}

static REGISTRY: LazyLock<HighlightRegistry> = LazyLock::new(HighlightRegistry::new);

pub fn registry() -> &'static HighlightRegistry {
    &REGISTRY
}
