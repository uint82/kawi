use tree_sitter::wasmtime::{Config, Engine};
use tree_sitter::{Language, Parser, WasmStore};

use crate::abi::check_abi;
use crate::error::LoadError;
use crate::hash::verify_sha256;

pub struct GrammarSpec<'a> {
    pub name: &'a str,
    pub sha256: &'a str,
}

pub struct GrammarEngine {
    engine: Engine,
}

impl GrammarEngine {
    pub fn new() -> Result<Self, LoadError> {
        let engine = Engine::new(&engine_config())
            .map_err(|error| LoadError::Engine(error.to_string()))?;
        Ok(Self { engine })
    }

    pub fn load(&self, spec: &GrammarSpec<'_>, wasm: &[u8]) -> Result<Grammar, LoadError> {
        verify_sha256(wasm, spec.sha256)?;
        let mut store = WasmStore::new(&self.engine)?;
        let language = store.load_language(spec.name, wasm)?;
        check_abi(language.abi_version())?;
        Ok(Grammar {
            engine: self.engine.clone(),
            language,
        })
    }
}

pub struct Grammar {
    engine: Engine,
    language: Language,
}

impl Grammar {
    pub fn language(&self) -> &Language {
        &self.language
    }

    pub fn abi_version(&self) -> usize {
        self.language.abi_version()
    }

    pub fn is_wasm(&self) -> bool {
        self.language.is_wasm()
    }

    pub fn parser(&self) -> Result<Parser, LoadError> {
        let mut parser = Parser::new();
        self.configure_parser(&mut parser)?;
        Ok(parser)
    }

    pub fn configure_parser(&self, parser: &mut Parser) -> Result<(), LoadError> {
        let store = WasmStore::new(&self.engine)?;
        parser.set_wasm_store(store)?;
        parser.set_language(&self.language)?;
        Ok(())
    }
}

// The tree-sitter stdlib needs bulk memory, and wasmparser only accepts the
// overlong call_indirect encoding emitted by wasi-sdk while reference types are
// enabled. Those two stay on. Everything else is off because grammar modules do
// not need it, and a smaller feature set is a smaller sandbox.
fn engine_config() -> Config {
    let mut config = Config::new();
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
