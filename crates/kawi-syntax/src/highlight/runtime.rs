use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tree_sitter::Query;
use tree_sitter_highlight::HighlightConfiguration;

use crate::external::GrammarLanguage;
use crate::language::LanguageId;
use crate::registry::{registry, SyntaxHighlight};

mod source;

pub(crate) use source::GrammarSource;

pub(crate) struct Runtime {
    pub(crate) config: HighlightConfiguration,
    pub(crate) captures: Vec<Option<SyntaxHighlight>>,
    pub(crate) injections: Option<Query>,
    pub(crate) locals: Option<Query>,
    pub(crate) non_local: Vec<bool>,
    pub(crate) priorities: Vec<i32>,
    source: GrammarSource,
}

impl Runtime {
    pub(crate) fn new(language: LanguageId) -> Option<Self> {
        let source = GrammarSource::from_language(language);
        if source.is_external() {
            Self::from_external(source).or_else(|| Self::from_native(language))
        } else {
            Self::from_native(language)
        }
    }

    pub(crate) fn source(&self) -> &GrammarSource {
        &self.source
    }

    fn from_native(language: LanguageId) -> Option<Self> {
        let source = GrammarSource::Native(language);
        let grammar_language = language.language();
        let locals = language.locals();
        let highlights = language.highlights();
        let config = build_configuration(
            grammar_language.clone(),
            language.name(),
            &highlights,
            locals,
        )
        .or_else(|| {
            build_configuration(
                grammar_language.clone(),
                language.name(),
                &language.bundled_highlights(),
                locals,
            )
        })?;
        Self::finish(
            source,
            config,
            grammar_language,
            language.injections().as_ref(),
            locals,
        )
    }

    fn from_external(source: GrammarSource) -> Option<Self> {
        let GrammarSource::External { grammar, .. } = &source else {
            return None;
        };
        source.parser()?;
        let name = source.name().to_string();
        let highlights = grammar.highlights().unwrap_or("").to_string();
        let injections = grammar.injections().unwrap_or("").to_string();
        let locals = grammar.locals().unwrap_or("").to_string();
        let grammar_language = source.language();
        let config = build_configuration(grammar_language.clone(), &name, &highlights, &locals)?;
        Self::finish(source, config, grammar_language, &injections, &locals)
    }

    fn finish(
        source: GrammarSource,
        mut config: HighlightConfiguration,
        language: GrammarLanguage,
        injections: &str,
        locals: &str,
    ) -> Option<Self> {
        config.configure(registry().names());
        let captures = capture_map(&config);
        let non_local = pattern_map(&config, is_non_local_pattern);
        let priorities = pattern_map(&config, pattern_priority);
        Some(Self {
            config,
            captures,
            injections: build_injection_query(&language, injections),
            locals: build_locals_query(&language, locals),
            non_local,
            priorities,
            source,
        })
    }
}

fn capture_map(config: &HighlightConfiguration) -> Vec<Option<SyntaxHighlight>> {
    config
        .names()
        .iter()
        .map(|name| resolve_capture(name))
        .collect()
}

fn pattern_map<T>(config: &HighlightConfiguration, map: fn(&Query, usize) -> T) -> Vec<T> {
    (0..config.query.pattern_count())
        .map(|pattern| map(&config.query, pattern))
        .collect()
}

#[derive(Clone)]
pub(crate) struct RuntimeCache {
    entries: Arc<Mutex<HashMap<LanguageId, Option<Arc<Runtime>>>>>,
}

impl RuntimeCache {
    pub(crate) fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn get(&self, language: LanguageId) -> Option<Arc<Runtime>> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        entries
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
    language: GrammarLanguage,
    name: &str,
    highlights: &str,
    locals: &str,
) -> Option<HighlightConfiguration> {
    HighlightConfiguration::new(language, name, highlights, "", locals).ok()
}

fn build_injection_query(language: &GrammarLanguage, injections: &str) -> Option<Query> {
    if injections.is_empty() {
        return None;
    }
    Query::new(language, injections).ok()
}

fn build_locals_query(language: &GrammarLanguage, locals: &str) -> Option<Query> {
    if locals.is_empty() {
        return None;
    }
    Query::new(language, locals).ok()
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
    use crate::external::{ExternalGrammar, GrammarParser};

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
                    build_locals_query(&language.language(), language.locals()).is_some(),
                    "{language:?}"
                );
            }
        }
    }

    #[test]
    fn shared_cache_reuses_a_runtime_across_threads() {
        let cache = RuntimeCache::new();
        let first = cache.get(LanguageId::Rust).expect("runtime");
        let other = cache.clone();
        let second = std::thread::spawn(move || other.get(LanguageId::Rust))
            .join()
            .expect("thread")
            .expect("runtime");
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn native_runtime_exposes_a_parser() {
        let runtime = Runtime::new(LanguageId::Rust).expect("runtime");
        assert!(!runtime.source().is_external());
        assert!(runtime.source().parser().is_some());
    }

    struct FakeGrammar {
        has_parser: bool,
        highlights: &'static str,
    }

    impl ExternalGrammar for FakeGrammar {
        fn name(&self) -> &str {
            "adl"
        }

        fn prepare(&self) -> Result<(), String> {
            Ok(())
        }

        fn language(&self) -> Option<GrammarLanguage> {
            Some(tree_sitter_adl::LANGUAGE.into())
        }

        fn parser(&self) -> Option<GrammarParser> {
            if !self.has_parser {
                return None;
            }
            let mut parser = GrammarParser::new();
            parser
                .set_language(&tree_sitter_adl::LANGUAGE.into())
                .ok()?;
            Some(parser)
        }

        fn highlights(&self) -> Option<&str> {
            Some(self.highlights)
        }

        fn injections(&self) -> Option<&str> {
            None
        }

        fn locals(&self) -> Option<&str> {
            None
        }
    }

    fn external_runtime(has_parser: bool, highlights: &'static str) -> Option<Runtime> {
        let source = GrammarSource::External {
            grammar: Arc::new(FakeGrammar {
                has_parser,
                highlights,
            }),
            language: tree_sitter_adl::LANGUAGE.into(),
        };
        Runtime::from_external(source)
    }

    #[test]
    fn external_runtime_uses_the_pack_queries() {
        let runtime = external_runtime(true, "(comment) @comment").expect("external runtime");
        assert!(runtime.source().is_external());
        assert_eq!(runtime.config.language_name, "adl");
        assert_eq!(runtime.config.query.pattern_count(), 1);
    }

    #[test]
    fn external_runtime_without_a_parser_is_rejected() {
        assert!(external_runtime(false, "(comment) @comment").is_none());
    }

    #[test]
    fn external_runtime_with_a_broken_query_is_rejected() {
        assert!(external_runtime(true, "(").is_none());
    }
}
