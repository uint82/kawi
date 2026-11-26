use tree_sitter::wasmtime::{Config, Engine};
use tree_sitter::{Language, Parser, Query, WasmStore};

use crate::abi;
use crate::error::GrammarError;
use crate::pack::Pack;

#[derive(Debug)]
pub struct GrammarLoader {
    engine: Engine,
}

#[derive(Debug)]
pub struct LoadedGrammar {
    engine: Engine,
    language: Language,
    highlights: String,
    injections: String,
    locals: String,
}

impl GrammarLoader {
    pub fn new() -> Result<Self, GrammarError> {
        let engine = Engine::new(&engine_config())
            .map_err(|error| GrammarError::Engine(error.to_string()))?;
        Ok(Self { engine })
    }

    pub fn load(&self, pack: &Pack) -> Result<LoadedGrammar, GrammarError> {
        let contents = pack.load_contents()?;
        let mut store = WasmStore::new(&self.engine)?;
        let language = store.load_language(&pack.manifest().id, &contents.wasm)?;
        abi::check_abi(language.abi_version())?;
        pack.manifest().ensure_abi(language.abi_version())?;
        validate_query(&language, &contents.highlights, "highlights")?;
        validate_query(&language, &contents.injections, "injections")?;
        validate_query(&language, &contents.locals, "locals")?;
        Ok(LoadedGrammar {
            engine: self.engine.clone(),
            language,
            highlights: contents.highlights,
            injections: contents.injections,
            locals: contents.locals,
        })
    }
}

impl LoadedGrammar {
    pub fn language(&self) -> Language {
        self.language.clone()
    }

    pub fn abi_version(&self) -> usize {
        self.language.abi_version()
    }

    pub fn parser(&self) -> Result<Parser, GrammarError> {
        let mut parser = Parser::new();
        self.configure_parser(&mut parser)?;
        Ok(parser)
    }

    pub fn configure_parser(&self, parser: &mut Parser) -> Result<(), GrammarError> {
        let store = WasmStore::new(&self.engine)?;
        parser.set_wasm_store(store)?;
        parser.set_language(&self.language)?;
        Ok(())
    }

    pub fn highlights(&self) -> &str {
        &self.highlights
    }

    pub fn injections(&self) -> &str {
        &self.injections
    }

    pub fn locals(&self) -> &str {
        &self.locals
    }
}

fn validate_query(
    language: &Language,
    source: &str,
    kind: &'static str,
) -> Result<(), GrammarError> {
    if source.is_empty() {
        return Ok(());
    }
    Query::new(language, source)
        .map(|_| ())
        .map_err(|source| GrammarError::Query { kind, source })
}

// The tree-sitter stdlib needs bulk memory. wasmparser only accepts the
// overlong call_indirect encoding wasi-sdk emits while reference types are on.
// Everything else stays off; a grammar module has no use for it.
fn engine_config() -> Config {
    let mut config = Config::new();
    config.wasm_bulk_memory(true);
    config.wasm_reference_types(true);
    config.wasm_simd(false);
    config.wasm_relaxed_simd(false);
    config.wasm_multi_memory(false);
    config.wasm_memory64(false);
    config.wasm_tail_call(false);
    config.wasm_function_references(false);
    config.wasm_gc(false);
    config.wasm_multi_value(false);
    config.wasm_branch_hinting(false);
    config.wasm_extended_const(false);
    config.wasm_wide_arithmetic(false);
    config
}
