use tree_sitter_highlight::HighlightConfiguration;

use crate::language::LanguageId;
use crate::registry::{registry, SyntaxHighlight};

pub(crate) struct Runtime {
    pub(crate) config: HighlightConfiguration,
    pub(crate) captures: Vec<Option<SyntaxHighlight>>,
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
            .map(|name| registry().resolve(name))
            .collect();
        Some(Self { config, captures })
    }
}

fn build_configuration(language: LanguageId, highlights: &str) -> Option<HighlightConfiguration> {
    HighlightConfiguration::new(language.language(), language.name(), highlights, "", "").ok()
}
