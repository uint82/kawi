use kawi_plugin::{
    ApplyEditParams, Capability, DiagnosticSummary, Outcome, Position, ProtocolTextEdit,
    SetDiagnosticsParams, UiMessageParams, MAX_DIAGNOSTIC_CODE_BYTES, MAX_DIAGNOSTIC_MESSAGE_BYTES,
    MAX_PLUGIN_DIAGNOSTICS,
};

use super::*;

fn bit(capability: Capability) -> u16 {
    let index = Capability::ALL
        .iter()
        .position(|candidate| *candidate == capability)
        .expect("capability");
    1 << index
}

fn grants(capability: Capability) -> CapabilityRegistry {
    registry(bit(capability))
}

fn edit(revision: u64, line: usize, column: usize) -> ApplyEditParams {
    ApplyEditParams {
        buffer: 1,
        revision,
        edits: vec![ProtocolTextEdit {
            start: Position {
                line,
                byte_column: column,
            },
            end: Position {
                line,
                byte_column: column,
            },
            text: "x".to_string(),
        }],
    }
}

fn diagnostic(message: String, code: Option<String>) -> DiagnosticSummary {
    DiagnosticSummary {
        start: Position {
            line: 0,
            byte_column: 0,
        },
        end: Position {
            line: 0,
            byte_column: 0,
        },
        severity: "error".to_string(),
        message,
        source: None,
        code,
    }
}

#[test]
fn position_oracle_rejects_mid_codepoint_columns() {
    let text = Rope::from_str("é\n");
    assert!(position_is_valid(&text, 0, 0));
    assert!(position_is_valid(&text, 0, 2));
    assert!(!position_is_valid(&text, 0, 1));
    assert!(!position_is_valid(&text, 2, 0));
    assert!(!position_is_valid(&text, 0, 4));
}

#[test]
fn denied_call_does_not_apply_edit() {
    let registry = registry(0);
    let mut editor = MockEditor::new();
    let params = serde_json::to_value(edit(0, 0, 0)).unwrap();
    assert!(matches!(
        editor.dispatch(&registry, "editor.apply_edit", params),
        Outcome::Denied(_)
    ));
    assert_eq!(editor.effects(), 0);
}

#[test]
fn stale_revision_does_not_apply_edit() {
    let registry = grants(Capability::EditorWrite);
    let mut editor = MockEditor::new();
    let params = serde_json::to_value(edit(1, 0, 0)).unwrap();
    assert_eq!(
        editor.dispatch(&registry, "editor.apply_edit", params),
        Outcome::Protocol
    );
    assert_eq!(editor.effects(), 0);
}

#[test]
fn mid_codepoint_position_does_not_apply_edit() {
    let registry = grants(Capability::EditorWrite);
    let mut editor = MockEditor::new();
    let params = serde_json::to_value(edit(0, 0, 1)).unwrap();
    assert_eq!(
        editor.dispatch(&registry, "editor.apply_edit", params),
        Outcome::Protocol
    );
    assert_eq!(editor.effects(), 0);
}

#[test]
fn valid_edit_updates_text_and_revision() {
    let registry = registry(bit(Capability::EditorWrite) | bit(Capability::EditorRead));
    let mut editor = MockEditor::new();
    let params = serde_json::to_value(edit(0, 0, 0)).unwrap();
    assert_eq!(
        editor.dispatch(&registry, "editor.apply_edit", params),
        Outcome::Success
    );
    assert_eq!(editor.effects(), 1);
    let state = editor.dispatch(
        &registry,
        "editor.get_state",
        serde_json::json!({"include_text": true}),
    );
    assert_eq!(state, Outcome::Success);
}

#[test]
fn diagnostics_reject_too_many_items() {
    let registry = grants(Capability::DiagnosticsPublish);
    let mut editor = MockEditor::new();
    let params = SetDiagnosticsParams {
        buffer: 1,
        revision: 0,
        diagnostics: vec![diagnostic(String::new(), None); MAX_PLUGIN_DIAGNOSTICS + 1],
    };
    assert_eq!(
        editor.dispatch(
            &registry,
            "editor.set_diagnostics",
            serde_json::to_value(params).unwrap()
        ),
        Outcome::Protocol
    );
    assert_eq!(editor.effects(), 0);
}

#[test]
fn diagnostics_reject_oversized_fields() {
    let registry = grants(Capability::DiagnosticsPublish);
    let mut editor = MockEditor::new();
    let params = SetDiagnosticsParams {
        buffer: 1,
        revision: 0,
        diagnostics: vec![diagnostic(
            "x".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES + 1),
            Some("x".repeat(MAX_DIAGNOSTIC_CODE_BYTES + 1)),
        )],
    };
    assert_eq!(
        editor.dispatch(
            &registry,
            "editor.set_diagnostics",
            serde_json::to_value(params).unwrap()
        ),
        Outcome::Protocol
    );
    assert_eq!(editor.effects(), 0);
}

#[test]
fn diagnostics_accept_boundary_sizes() {
    let registry = grants(Capability::DiagnosticsPublish);
    let mut editor = MockEditor::new();
    let params = SetDiagnosticsParams {
        buffer: 1,
        revision: 0,
        diagnostics: vec![diagnostic(
            "x".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES),
            Some("x".repeat(MAX_DIAGNOSTIC_CODE_BYTES)),
        )],
    };
    assert_eq!(
        editor.dispatch(
            &registry,
            "editor.set_diagnostics",
            serde_json::to_value(params).unwrap()
        ),
        Outcome::Success
    );
    assert_eq!(editor.effects(), 1);
}

#[test]
fn message_sanitizes_control_characters() {
    let registry = grants(Capability::UiMessage);
    let mut editor = MockEditor::new();
    let params = UiMessageParams {
        message: "safe\u{1b}[31mred\u{1b}[0m\u{7}".to_string(),
    };
    assert_eq!(
        editor.dispatch(
            &registry,
            "ui.message",
            serde_json::to_value(params).unwrap()
        ),
        Outcome::Success
    );
    assert_eq!(editor.effects(), 1);
}

#[test]
fn unknown_method_is_denied_without_effect() {
    let registry = grants(Capability::EditorWrite);
    let mut editor = MockEditor::new();
    assert!(matches!(
        editor.dispatch(&registry, "unknown.method", serde_json::Value::Null),
        Outcome::Denied(_)
    ));
    assert_eq!(editor.effects(), 0);
}
