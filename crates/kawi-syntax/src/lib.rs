mod diff;
mod highlight;
mod language;
mod limits;
mod registry;
mod remap;
mod rows;
mod service;
mod snippet;
mod span;
mod text;

pub use language::{reload_user_queries, set_user_query_dir, user_query_warnings, LanguageId};
pub use limits::{file_size_supported, MAX_FILE_BYTES, MAX_SNIPPET_BYTES};
pub use registry::{registry, HighlightRegistry, SyntaxHighlight};
pub use remap::shift_spans;
pub use service::SyntaxService;
pub use snippet::{highlight_snippet, highlight_snippet_with_cancel};
pub use span::{SnippetSpan, SyntaxEdit, SyntaxKey, SyntaxRequest, SyntaxResult, SyntaxSpan};
