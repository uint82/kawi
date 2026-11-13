mod diff;
mod highlight;
mod language;
mod limits;
mod registry;
mod rows;
mod service;
mod snippet;
mod span;
mod text;

pub use language::{set_user_query_dir, LanguageId};
pub use limits::{file_size_supported, MAX_FILE_BYTES, MAX_SNIPPET_BYTES};
pub use registry::{registry, HighlightRegistry, SyntaxHighlight};
pub use service::SyntaxService;
pub use snippet::{highlight_snippet, highlight_snippet_with_cancel};
pub use span::{SnippetSpan, SyntaxEdit, SyntaxKey, SyntaxRequest, SyntaxResult, SyntaxSpan};
