use std::error::Error;
use std::fmt;

use tree_sitter::{LanguageError, WasmError};

use crate::abi::AbiError;
use crate::hash::HashError;

#[derive(Debug)]
pub enum LoadError {
    Engine(String),
    Hash(HashError),
    Wasm(WasmError),
    Abi(AbiError),
    Language(LanguageError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Engine(message) => {
                write!(formatter, "failed to create the Wasm engine: {message}")
            }
            Self::Hash(error) => write!(formatter, "invalid grammar hash: {error}"),
            Self::Wasm(error) => write!(formatter, "failed to load the Wasm grammar: {error}"),
            Self::Abi(error) => write!(formatter, "unsupported grammar ABI: {error}"),
            Self::Language(error) => write!(formatter, "failed to configure the parser: {error}"),
        }
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Engine(_) => None,
            Self::Hash(error) => Some(error),
            Self::Wasm(error) => Some(error),
            Self::Abi(error) => Some(error),
            Self::Language(error) => Some(error),
        }
    }
}

impl From<HashError> for LoadError {
    fn from(error: HashError) -> Self {
        Self::Hash(error)
    }
}

impl From<WasmError> for LoadError {
    fn from(error: WasmError) -> Self {
        Self::Wasm(error)
    }
}

impl From<AbiError> for LoadError {
    fn from(error: AbiError) -> Self {
        Self::Abi(error)
    }
}

impl From<LanguageError> for LoadError {
    fn from(error: LanguageError) -> Self {
        Self::Language(error)
    }
}
