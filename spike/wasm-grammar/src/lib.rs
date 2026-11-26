//! Loads tree-sitter grammars compiled to WebAssembly.
//!
//! This crate is a spike for the wasm grammar pack design. It stays outside
//! the workspace until the Wasmtime version conflict is resolved, so the
//! shipped editor does not link two Wasm engines.

mod abi;
mod error;
mod grammar;
mod hash;

pub use abi::{check_abi, AbiError, MAX_ABI, MIN_ABI};
pub use error::LoadError;
pub use grammar::{Grammar, GrammarEngine, GrammarSpec};
pub use hash::{sha256_hex, verify_sha256, HashError};
