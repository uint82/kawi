use std::sync::Arc;

use crate::external::{external_grammar, ExternalGrammar, GrammarLanguage, GrammarParser};
use crate::language::LanguageId;

#[derive(Clone)]
pub(crate) enum GrammarSource {
    Native(LanguageId),
    External {
        grammar: Arc<dyn ExternalGrammar>,
        language: GrammarLanguage,
    },
}

impl GrammarSource {
    pub(crate) fn from_language(language: LanguageId) -> Self {
        if let Some(grammar) = external_grammar(language) {
            if grammar.prepare().is_ok() {
                if let Some(grammar_language) = grammar.language() {
                    return Self::External {
                        grammar,
                        language: grammar_language,
                    };
                }
            }
        }
        Self::Native(language)
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Native(language) => language.name(),
            Self::External { grammar, .. } => grammar.name(),
        }
    }

    pub(crate) fn language(&self) -> GrammarLanguage {
        match self {
            Self::Native(language) => language.language(),
            Self::External { language, .. } => language.clone(),
        }
    }

    pub(crate) fn parser(&self) -> Option<GrammarParser> {
        match self {
            Self::Native(language) => {
                let mut parser = GrammarParser::new();
                parser.set_language(&language.language()).ok()?;
                Some(parser)
            }
            Self::External { grammar, .. } => grammar.parser(),
        }
    }

    pub(crate) fn is_external(&self) -> bool {
        matches!(self, Self::External { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_source_creates_a_parser() {
        let source = GrammarSource::Native(LanguageId::Rust);
        assert!(!source.is_external());
        assert!(source.parser().is_some());
    }

    #[test]
    fn native_source_uses_the_language_name() {
        assert_eq!(GrammarSource::Native(LanguageId::Adl).name(), "adl");
    }
}
