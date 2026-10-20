mod diff;
mod group;
mod highlight;
mod language;
mod limits;
mod rows;
mod service;
mod span;

pub use group::SyntaxGroup;
pub use language::LanguageId;
pub use limits::{file_size_supported, MAX_FILE_BYTES};
pub use service::SyntaxService;
pub use span::{SyntaxKey, SyntaxRequest, SyntaxResult, SyntaxSpan};
