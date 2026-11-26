use std::collections::HashMap;
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

use crate::language::LanguageId;

pub type GrammarLanguage = tree_sitter::Language;
pub type GrammarParser = tree_sitter::Parser;

/// A grammar supplied by a Wasm pack instead of the built-in table.
///
/// `prepare` runs before any getter. Implementations should make it
/// idempotent: the runtime may ask more than once per process.
pub trait ExternalGrammar: Send + Sync {
    fn name(&self) -> &str;
    fn prepare(&self) -> Result<(), String>;
    fn language(&self) -> Option<GrammarLanguage>;
    fn parser(&self) -> Option<GrammarParser>;
    fn highlights(&self) -> Option<&str>;
    fn injections(&self) -> Option<&str>;
    fn locals(&self) -> Option<&str>;
}

static EXTERNAL_GRAMMARS: LazyLock<RwLock<HashMap<LanguageId, Arc<dyn ExternalGrammar>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn register_external_grammar(language: LanguageId, grammar: Arc<dyn ExternalGrammar>) {
    EXTERNAL_GRAMMARS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(language, grammar);
}

pub fn clear_external_grammars() {
    EXTERNAL_GRAMMARS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
}

pub(crate) fn external_grammar(language: LanguageId) -> Option<Arc<dyn ExternalGrammar>> {
    EXTERNAL_GRAMMARS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&language)
        .cloned()
}
