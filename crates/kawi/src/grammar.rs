use std::path::Path;
use std::sync::{Arc, OnceLock};

use kawi_grammar::{GrammarError, GrammarLoader, GrammarStore, LoadedGrammar, Pack, PackRoot};
use kawi_syntax::{
    register_external_grammar, ExternalGrammar, GrammarLanguage, GrammarParser, LanguageId,
};

#[cfg(test)]
mod tests;

pub(crate) fn register(root: &Path) {
    let store = GrammarStore::new(vec![PackRoot::user(root)]);
    let packs = store.best_packs();
    if packs.is_empty() {
        return;
    }
    let loader = match GrammarLoader::new() {
        Ok(loader) => Arc::new(loader),
        Err(error) => {
            eprintln!("kawi: grammar loader: {error}");
            return;
        }
    };
    for pack in packs {
        register_pack(loader.clone(), pack);
    }
}

fn register_pack(loader: Arc<GrammarLoader>, pack: Pack) {
    let name = pack.manifest().language.clone();
    let Some(language) = LanguageId::from_alias(&name) else {
        eprintln!(
            "kawi: grammar pack {} targets unknown language {name}",
            pack.manifest().id
        );
        return;
    };
    register_external_grammar(language, Arc::new(LazyGrammar::new(loader, pack)));
}

pub(crate) fn load_pack(
    loader: &GrammarLoader,
    pack: &Pack,
) -> Result<LoadedGrammar, GrammarError> {
    pack.manifest()
        .ensure_host_compatible(env!("CARGO_PKG_VERSION"))?;
    loader.load(pack)
}

struct LazyGrammar {
    loader: Arc<GrammarLoader>,
    pack: Pack,
    loaded: OnceLock<Result<LoadedGrammar, String>>,
}

impl LazyGrammar {
    fn new(loader: Arc<GrammarLoader>, pack: Pack) -> Self {
        Self {
            loader,
            pack,
            loaded: OnceLock::new(),
        }
    }

    fn loaded(&self) -> Option<&LoadedGrammar> {
        self.ensure_loaded().ok()
    }

    fn ensure_loaded(&self) -> Result<&LoadedGrammar, &str> {
        self.loaded
            .get_or_init(|| {
                load_pack(&self.loader, &self.pack).map_err(|error| {
                    eprintln!("kawi: grammar pack {}: {error}", self.pack.manifest().id);
                    error.to_string()
                })
            })
            .as_ref()
            .map_err(String::as_str)
    }
}

impl ExternalGrammar for LazyGrammar {
    fn name(&self) -> &str {
        self.pack.manifest().id.as_str()
    }

    fn prepare(&self) -> Result<(), String> {
        self.ensure_loaded()
            .map(|_| ())
            .map_err(ToString::to_string)
    }

    fn language(&self) -> Option<GrammarLanguage> {
        Some(self.loaded()?.language())
    }

    fn parser(&self) -> Option<GrammarParser> {
        self.loaded()?.parser().ok()
    }

    fn highlights(&self) -> Option<&str> {
        Some(self.loaded()?.highlights())
    }

    fn injections(&self) -> Option<&str> {
        non_empty(self.loaded()?.injections())
    }

    fn locals(&self) -> Option<&str> {
        non_empty(self.loaded()?.locals())
    }
}

fn non_empty(text: &str) -> Option<&str> {
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
