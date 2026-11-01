use kawi_plugin::{
    Decoration, DecorationStyle, Position, SetDecorationsParams, SetStatuslineParams,
    StatuslineSegment, INVALID_PARAMS, STALE_REVISION,
};
use serde_json::json;

use super::common::*;

fn set_decorations(
    app: &mut crate::app::App,
    decorations: Vec<Decoration>,
) -> Result<serde_json::Value, kawi_plugin::RpcError> {
    let params = SetDecorationsParams {
        buffer: app.editor.buffer_number() as u64,
        revision: app.editor.buffer_revision(),
        decorations,
    };
    app.handle_plugin_request(
        "visual",
        "editor.set_decorations",
        serde_json::to_value(params).unwrap(),
    )
}

#[test]
fn stores_highlights_virtual_text_and_signs() {
    let mut app = app();
    set_decorations(
        &mut app,
        vec![
            Decoration::Highlight {
                start: Position {
                    line: 0,
                    byte_column: 0,
                },
                end: Position {
                    line: 0,
                    byte_column: 4,
                },
                style: DecorationStyle::Info,
            },
            Decoration::VirtualText {
                line: 0,
                text: "note".to_string(),
                style: DecorationStyle::Muted,
            },
            Decoration::Sign {
                line: 0,
                text: "!".to_string(),
                style: DecorationStyle::Error,
            },
        ],
    )
    .unwrap();
    let buffer = app.editor.buffer_number();
    assert_eq!(app.plugin_decorations.spans(buffer, &[0]).len(), 1);
    assert_eq!(
        app.plugin_decorations.virtual_texts(buffer, &[0])[0].len(),
        1
    );
    assert!(app.plugin_decorations.has_signs(buffer));
}

#[test]
fn rejects_stale_revisions_and_invalid_sign_text() {
    let mut app = app();
    let error = app
        .handle_plugin_request(
            "visual",
            "editor.set_decorations",
            json!({
                "buffer": app.editor.buffer_number(),
                "revision": app.editor.buffer_revision() + 1,
                "decorations": []
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, STALE_REVISION);
    let error = set_decorations(
        &mut app,
        vec![Decoration::Sign {
            line: 0,
            text: "ab".to_string(),
            style: DecorationStyle::Error,
        }],
    )
    .unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);
}

#[test]
fn stores_and_replaces_statusline_segments() {
    let mut app = app();
    let set = |app: &mut crate::app::App, text: &str| {
        let params = SetStatuslineParams {
            segments: vec![StatuslineSegment {
                text: text.to_string(),
                style: DecorationStyle::Info,
                priority: 0,
            }],
        };
        app.handle_plugin_request(
            "visual",
            "ui.set_statusline",
            serde_json::to_value(params).unwrap(),
        )
        .unwrap();
    };
    set(&mut app, "one");
    set(&mut app, "two");
    let segments = app.plugin_statusline.segments();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "two");
    app.plugin_statusline.remove_plugin("visual");
    assert!(app.plugin_statusline.segments().is_empty());
}

#[test]
fn rejects_oversized_statusline_payloads() {
    let mut app = app();
    let params = SetStatuslineParams {
        segments: (0..17)
            .map(|_| StatuslineSegment {
                text: "x".to_string(),
                style: DecorationStyle::Info,
                priority: 0,
            })
            .collect(),
    };
    let error = app
        .handle_plugin_request(
            "visual",
            "ui.set_statusline",
            serde_json::to_value(params).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);
}
