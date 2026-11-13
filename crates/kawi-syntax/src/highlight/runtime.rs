use std::collections::HashMap;
use std::sync::Arc;

use tree_sitter::Query;
use tree_sitter_highlight::HighlightConfiguration;

use crate::language::LanguageId;
use crate::registry::{registry, SyntaxHighlight};

pub(crate) struct Runtime {
    pub(crate) config: HighlightConfiguration,
    pub(crate) captures: Vec<Option<SyntaxHighlight>>,
    pub(crate) injections: Option<Query>,
}

impl Runtime {
    pub(crate) fn new(language: LanguageId) -> Option<Self> {
        let highlights = language.highlights();
        let mut config = build_configuration(language, highlights)
            .or_else(|| build_configuration(language, &language.bundled_highlights()))?;
        config.configure(registry().names());
        let captures = config
            .names()
            .iter()
            .map(|name| resolve_capture(name))
            .collect();
        Some(Self {
            config,
            captures,
            injections: build_injection_query(language),
        })
    }
}

pub(crate) struct RuntimeCache {
    entries: HashMap<LanguageId, Option<Arc<Runtime>>>,
}

impl RuntimeCache {
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn get(&mut self, language: LanguageId) -> Option<Arc<Runtime>> {
        self.entries
            .entry(language)
            .or_insert_with(|| Runtime::new(language).map(Arc::new))
            .clone()
    }
}

impl Default for RuntimeCache {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_capture(name: &str) -> Option<SyntaxHighlight> {
    if name.starts_with('_') {
        return None;
    }
    registry().resolve(name)
}

fn build_configuration(language: LanguageId, highlights: &str) -> Option<HighlightConfiguration> {
    HighlightConfiguration::new(language.language(), language.name(), highlights, "", "").ok()
}

fn build_injection_query(language: LanguageId) -> Option<Query> {
    let injections = language.injections();
    if injections.is_empty() {
        return None;
    }
    Query::new(&language.language(), &injections).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_captures_are_not_highlighted() {
        assert!(resolve_capture("_attr").is_none());
        assert!(resolve_capture("keyword").is_some());
    }
}
