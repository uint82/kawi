use super::super::ApiSignatureMetadata;
use super::{optional, required, signature};

pub(super) fn lookup(name: &str) -> Option<ApiSignatureMetadata> {
    Some(match name {
        "buffer.apply_edit" => signature(
            parameters![
                required("buffer", "integer"),
                required("revision", "integer"),
                required("edits", "TextEdit[]"),
            ],
            "ApplyEditResult",
        ),
        "buffer.close" => signature(
            parameters![required("buffer", "integer"), optional("force", "boolean")],
            "null",
        ),
        "buffer.create" => signature(parameters![optional("text", "string")], "BufferState"),
        "buffer.get" => signature(
            parameters![
                required("buffer", "integer"),
                optional("include_text", "boolean"),
            ],
            "BufferState",
        ),
        "buffer.list" => signature(parameters![], "BufferInfo[]"),
        "buffer.open" => signature(parameters![required("path", "string")], "BufferState"),
        "window.close" => signature(
            parameters![required("window", "integer"), optional("force", "boolean")],
            "WindowInfo",
        ),
        "window.focus" => signature(parameters![required("window", "integer")], "WindowInfo"),
        "window.get" => signature(parameters![required("window", "integer")], "WindowInfo"),
        "window.list" => signature(parameters![], "WindowInfo[]"),
        "window.open" => signature(
            parameters![
                required("buffer", "integer"),
                required("direction", "string"),
            ],
            "WindowInfo",
        ),
        _ => return None,
    })
}
