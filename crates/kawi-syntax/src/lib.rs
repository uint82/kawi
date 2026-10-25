mod diff;
mod group;
mod highlight;
mod language;
mod limits;
mod rows;
mod service;
mod snippet;
mod span;

pub use group::SyntaxGroup;
pub use language::LanguageId;
pub use limits::{file_size_supported, MAX_FILE_BYTES};
pub use service::SyntaxService;
pub use snippet::{highlight_snippet, highlight_snippet_with_cancel};
pub use span::{SnippetSpan, SyntaxKey, SyntaxRequest, SyntaxResult, SyntaxSpan};
