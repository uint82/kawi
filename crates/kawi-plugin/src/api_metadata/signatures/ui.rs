use super::super::ApiSignatureMetadata;
use super::{optional, required, signature};

pub(super) fn lookup(name: &str) -> Option<ApiSignatureMetadata> {
    Some(match name {
        "ui.confirm" => signature(
            parameters![required("title", "string"), optional("message", "string"),],
            "UiPromptOutcome",
        ),
        "ui.input" => signature(
            parameters![
                required("title", "string"),
                optional("placeholder", "string"),
                optional("value", "string"),
                optional("max_length", "integer"),
            ],
            "UiPromptOutcome",
        ),
        "ui.message" => signature(parameters![required("message", "string")], "null"),
        "ui.overlay_close" => signature(parameters![required("overlay", "integer")], "null"),
        "ui.overlay_open" => signature(
            parameters![
                required("title", "string"),
                required("lines", "OverlayLine[]"),
                optional("status", "string"),
                optional("anchor", "string"),
                optional("width", "integer"),
                optional("height", "integer"),
                optional("focus", "boolean"),
                optional("selected", "integer"),
                optional("scroll", "integer"),
            ],
            "OverlayOpenResult",
        ),
        "ui.overlay_update" => signature(
            parameters![
                required("overlay", "integer"),
                optional("title", "string"),
                optional("status", "string"),
                optional("anchor", "string"),
                optional("width", "integer"),
                optional("height", "integer"),
                optional("focus", "boolean"),
                optional("selected", "integer"),
                optional("scroll", "integer"),
                optional("lines", "OverlayLine[]"),
            ],
            "null",
        ),
        "ui.overlay_wait" => signature(parameters![required("overlay", "integer")], "OverlayEvent"),
        "ui.panel_close" => signature(parameters![required("panel", "integer")], "null"),
        "ui.panel_focus" => signature(parameters![required("panel", "integer")], "null"),
        "ui.panel_open" => signature(
            parameters![
                required("title", "string"),
                required("lines", "PanelLine[]"),
                optional("status", "string"),
                optional("height", "integer"),
                optional("focus", "boolean"),
            ],
            "PanelOpenResult",
        ),
        "ui.panel_update" => signature(
            parameters![
                required("panel", "integer"),
                optional("title", "string"),
                optional("status", "string"),
                optional("height", "integer"),
                optional("lines", "PanelLine[]"),
            ],
            "null",
        ),
        "ui.picker" => signature(
            parameters![
                required("title", "string"),
                required("items", "PickerItem[]"),
                optional("selected", "integer"),
            ],
            "PickerOutcome",
        ),
        "ui.progress_finish" => signature(parameters![required("id", "integer")], "null"),
        "ui.progress_start" => signature(
            parameters![
                required("title", "string"),
                optional("message", "string"),
                optional("fraction", "number"),
            ],
            "UiProgressOpenResult",
        ),
        "ui.progress_update" => signature(
            parameters![
                required("id", "integer"),
                optional("message", "string"),
                optional("fraction", "number"),
            ],
            "UiProgressUpdateResult",
        ),
        "ui.select" => signature(
            parameters![
                required("title", "string"),
                required("options", "string[]"),
                optional("selected", "integer"),
            ],
            "UiPromptOutcome",
        ),
        "ui.set_statusline" => signature(
            parameters![required("segments", "StatuslineSegment[]")],
            "null",
        ),
        _ => return None,
    })
}
