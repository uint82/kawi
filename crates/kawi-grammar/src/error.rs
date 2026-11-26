use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

use tree_sitter::{LanguageError, QueryError, WasmError};

use crate::abi::AbiError;

#[derive(Debug)]
pub enum GrammarError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Manifest {
        path: PathBuf,
        source: toml::de::Error,
    },
    InvalidManifest(String),
    UnsafePath(String),
    MissingFile(PathBuf),
    NotAFile(PathBuf),
    NotADirectory(PathBuf),
    Symlink(PathBuf),
    UnsupportedFileType(PathBuf),
    HashMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    UnsupportedFormat {
        found: u32,
        supported: u32,
    },
    UnsupportedKawi {
        required: String,
        current: String,
    },
    Abi {
        found: usize,
        min: usize,
        max: usize,
    },
    AbiRange {
        pack_min: usize,
        pack_max: usize,
        host_min: usize,
        host_max: usize,
    },
    Engine(String),
    Wasm(WasmError),
    Language(LanguageError),
    Query {
        kind: &'static str,
        source: QueryError,
    },
    TooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },
    InvalidUtf8 {
        path: PathBuf,
        source: std::string::FromUtf8Error,
    },
    Install(String),
}

impl GrammarError {
    pub(crate) fn io(path: &std::path::Path, source: io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for GrammarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Manifest { path, source } => {
                write!(formatter, "{}: {source}", path.display())
            }
            Self::InvalidManifest(reason) => write!(formatter, "invalid manifest: {reason}"),
            Self::UnsafePath(path) => write!(formatter, "unsafe path in pack: {path}"),
            Self::MissingFile(path) => write!(formatter, "missing pack file: {}", path.display()),
            Self::NotAFile(path) => write!(formatter, "not a file: {}", path.display()),
            Self::NotADirectory(path) => write!(formatter, "not a directory: {}", path.display()),
            Self::Symlink(path) => {
                write!(formatter, "refusing symlink: {}", path.display())
            }
            Self::UnsupportedFileType(path) => {
                write!(formatter, "unsupported file type: {}", path.display())
            }
            Self::HashMismatch {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "{}: expected sha256 {expected}, got {actual}",
                path.display()
            ),
            Self::UnsupportedFormat { found, supported } => write!(
                formatter,
                "pack format {found} is not supported, expected {supported}"
            ),
            Self::UnsupportedKawi { required, current } => write!(
                formatter,
                "pack requires kawi {required} or newer, this is {current}"
            ),
            Self::Abi { found, min, max } => write!(
                formatter,
                "grammar ABI {found} is outside the supported range {min}..={max}"
            ),
            Self::AbiRange {
                pack_min,
                pack_max,
                host_min,
                host_max,
            } => write!(
                formatter,
                "pack supports ABI {pack_min}..={pack_max}, host supports {host_min}..={host_max}"
            ),
            Self::Engine(message) => {
                write!(formatter, "failed to create the Wasm engine: {message}")
            }
            Self::Wasm(error) => write!(formatter, "failed to load the Wasm grammar: {error}"),
            Self::Language(error) => write!(formatter, "failed to configure the parser: {error}"),
            Self::Query { kind, source } => write!(formatter, "invalid {kind} query: {source}"),
            Self::TooLarge { path, size, limit } => write!(
                formatter,
                "{}: {size} bytes exceeds the {limit} byte limit",
                path.display()
            ),
            Self::InvalidUtf8 { path, source } => {
                write!(formatter, "{}: {source}", path.display())
            }
            Self::Install(reason) => write!(formatter, "cannot install grammar pack: {reason}"),
        }
    }
}

impl Error for GrammarError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Manifest { source, .. } => Some(source),
            Self::Wasm(error) => Some(error),
            Self::Language(error) => Some(error),
            Self::Query { source, .. } => Some(source),
            Self::InvalidUtf8 { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<AbiError> for GrammarError {
    fn from(error: AbiError) -> Self {
        Self::Abi {
            found: error.found,
            min: error.min,
            max: error.max,
        }
    }
}

impl From<WasmError> for GrammarError {
    fn from(error: WasmError) -> Self {
        Self::Wasm(error)
    }
}

impl From<LanguageError> for GrammarError {
    fn from(error: LanguageError) -> Self {
        Self::Language(error)
    }
}
