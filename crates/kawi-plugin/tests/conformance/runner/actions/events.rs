use serde_json::{json, Value};

pub(crate) fn event_payload(event: &str) -> Value {
    match event {
        "buffer_changed" => json!({
            "buffer": 1,
            "revision": 2,
            "previous_revision": 1,
            "origin": "user",
            "full_resync": false,
            "changes": [],
        }),
        "buffer_opened" => json!({
            "buffer": 1,
            "path": null,
            "language": null,
            "revision": 1,
        }),
        "file_opened" => json!({
            "buffer": 1,
            "path": "/tmp/conformance.rs",
            "language": "rust",
            "revision": 1,
        }),
        "buffer_closed" => json!({
            "buffer": 1,
            "path": null,
        }),
        "file_saved" => json!({
            "buffer": 1,
            "path": null,
            "revision": 1,
        }),
        "mode_changed" => json!({
            "buffer": 1,
            "from": "normal",
            "to": "insert",
        }),
        "diagnostics_changed" => json!({
            "buffer": 1,
            "path": "/tmp/conformance.rs",
            "revision": 1,
            "diagnostics": [],
        }),
        _ => panic!("unknown event: {event}"),
    }
}
