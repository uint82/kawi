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
    pub(crate) locals: Option<Query>,
    pub(crate) non_local: Vec<bool>,
    pub(crate) priorities: Vec<i32>,
}

impl Runtime {
    pub(crate) fn new(language: LanguageId) -> Option<Self> {
        let highlights = language.highlights();
        let locals = language.locals();
        let mut config = build_configuration(language, &highlights, locals)
            .or_else(|| build_configuration(language, &language.bundled_highlights(), locals))?;
        config.configure(registry().names());
        let captures = config
            .names()
            .iter()
            .map(|name| resolve_capture(name))
            .collect();
        let non_local = (0..config.query.pattern_count())
            .map(|pattern| is_non_local_pattern(&config.query, pattern))
            .collect();
        let priorities = (0..config.query.pattern_count())
            .map(|pattern| pattern_priority(&config.query, pattern))
            .collect();
        Some(Self {
            config,
            captures,
            injections: build_injection_query(language),
            locals: build_locals_query(language, locals),
            non_local,
            priorities,
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

fn build_configuration(
    language: LanguageId,
    highlights: &str,
    locals: &str,
) -> Option<HighlightConfiguration> {
    HighlightConfiguration::new(language.language(), language.name(), highlights, "", locals).ok()
}

fn build_injection_query(language: LanguageId) -> Option<Query> {
    let injections = language.injections();
    if injections.is_empty() {
        return None;
    }
    Query::new(&language.language(), &injections).ok()
}

fn build_locals_query(language: LanguageId, locals: &str) -> Option<Query> {
    if locals.is_empty() {
        return None;
    }
    Query::new(&language.language(), locals).ok()
}

fn is_non_local_pattern(query: &Query, pattern: usize) -> bool {
    query
        .property_predicates(pattern)
        .iter()
        .any(|(property, positive)| !*positive && property.key.as_ref() == "local")
}

fn pattern_priority(query: &Query, pattern: usize) -> i32 {
    query
        .property_settings(pattern)
        .iter()
        .find(|property| property.key.as_ref() == "priority")
        .and_then(|property| property.value.as_deref())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_captures_are_not_highlighted() {
        assert!(resolve_capture("_attr").is_none());
        assert!(resolve_capture("keyword").is_some());
    }

    #[test]
    fn priority_property_becomes_a_capture_priority() {
        let language = LanguageId::Rust.language();
        let query =
            Query::new(&language, "((identifier) @variable (#set! priority 10))").expect("query");
        assert_eq!(pattern_priority(&query, 0), 10);
    }

    #[test]
    fn locals_queries_load_for_supported_languages() {
        for language in LanguageId::ALL {
            if !language.locals().is_empty() {
                assert!(
                    build_locals_query(language, language.locals()).is_some(),
                    "{language:?}"
                );
            }
        }
    }
}
