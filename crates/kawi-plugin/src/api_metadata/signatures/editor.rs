use super::super::ApiSignatureMetadata;
use super::{optional, required, signature};

pub(super) fn lookup(name: &str) -> Option<ApiSignatureMetadata> {
    Some(match name {
        "command.list" => signature(parameters![], "RegisteredCommand[]"),
        "command.register" => signature(
            parameters![
                required("id", "string"),
                required("title", "string"),
                optional("description", "string"),
                optional("aliases", "string[]"),
            ],
            "RegisteredCommand",
        ),
        "command.unregister" => signature(parameters![required("id", "string")], "null"),
        "editor.apply_edit" => signature(
            parameters![
                required("buffer", "integer"),
                required("revision", "integer"),
                required("edits", "TextEdit[]"),
            ],
            "ApplyEditResult",
        ),
        "editor.emit_user_event" => signature(
            parameters![required("name", "string"), optional("payload", "string")],
            "null",
        ),
        "editor.execute" => signature(parameters![required("command", "string")], "null"),
        "editor.get_state" => signature(
            parameters![optional("include_text", "boolean")],
            "GetStateResult",
        ),
        "editor.list_commands" => signature(parameters![], "EditorListCommandsResult"),
        "editor.set_cursor" => signature(
            parameters![
                required("buffer", "integer"),
                required("cursor", "Position"),
            ],
            "GetStateResult",
        ),
        "editor.set_decorations" => signature(
            parameters![
                required("buffer", "integer"),
                required("revision", "integer"),
                required("decorations", "Decoration[]"),
                optional("scope", "string"),
            ],
            "null",
        ),
        "editor.set_diagnostics" => signature(
            parameters![
                required("buffer", "integer"),
                required("revision", "integer"),
                required("diagnostics", "DiagnosticSummary[]"),
            ],
            "null",
        ),
        "editor.set_mode" => signature(
            parameters![required("buffer", "integer"), required("mode", "string")],
            "GetStateResult",
        ),
        "editor.set_selection" => signature(
            parameters![
                required("buffer", "integer"),
                optional("selection", "Selection"),
            ],
            "GetStateResult",
        ),
        "keymap.list" => signature(parameters![], "RegisteredKeymap[]"),
        "keymap.remove" => signature(
            parameters![required("mode", "string"), required("keys", "string")],
            "null",
        ),
        "keymap.set" => signature(
            parameters![
                required("mode", "string"),
                required("keys", "string"),
                required("command", "string"),
                optional("description", "string"),
            ],
            "RegisteredKeymap",
        ),
        "options.get" => signature(parameters![required("name", "string")], "OptionInfo"),
        "options.list" => signature(parameters![optional("prefix", "string")], "OptionInfo[]"),
        "options.reset" => signature(parameters![required("name", "string")], "OptionInfo"),
        "options.set" => signature(
            parameters![required("name", "string"), required("value", "any")],
            "OptionInfo",
        ),
        _ => return None,
    })
}
