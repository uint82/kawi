#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use kawi_fuzz::host::{registry, MockEditor};
use kawi_plugin::{
    host_method_capability, is_host_method_allowed, sanitize, validate_diagnostics,
    MethodCapability, Outcome, SetDiagnosticsParams, MAX_DIAGNOSTIC_CODE_BYTES,
    MAX_DIAGNOSTIC_MESSAGE_BYTES, MAX_PLUGIN_DIAGNOSTICS,
};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

#[derive(Arbitrary, Debug)]
struct Input {
    grants: u16,
    calls: Vec<Call>,
}

#[derive(Debug)]
struct Call {
    method: u8,
    params: Vec<u8>,
}

impl<'a> Arbitrary<'a> for Call {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let method = u.arbitrary()?;
        let length = u.arbitrary::<u32>()? as usize;
        let params = u.bytes(length)?.to_vec();
        Ok(Self { method, params })
    }
}

fuzz_target!(|input: Input| {
    let registry = registry(input.grants);
    let mut editor = MockEditor::new();

    for call in &input.calls {
        let method = method_name(call.method);
        let params = serde_json::from_slice(&call.params).unwrap_or(Value::Null);
        let expected = expected_allowed(input.grants, method);
        assert_eq!(is_host_method_allowed(&registry, "fuzz", method), expected);

        let before = editor.effects();
        let outcome = editor.dispatch(&registry, method, params.clone());
        if !expected {
            assert!(matches!(outcome, Outcome::Denied(_)));
            assert_eq!(editor.effects(), before);
            continue;
        }

        check_diagnostics(method, &params);
        check_sanitize(method, &params);
        check_effect(method, &outcome, before, editor.effects());
    }
});

fn method_name(index: u8) -> &'static str {
    match index % 5 {
        0 => "editor.get_state",
        1 => "editor.apply_edit",
        2 => "editor.set_diagnostics",
        3 => "ui.message",
        _ => "unknown.method",
    }
}

fn expected_allowed(grants: u16, method: &str) -> bool {
    match host_method_capability(method) {
        MethodCapability::Required(capability) => {
            let index = kawi_plugin::Capability::ALL
                .iter()
                .position(|candidate| *candidate == capability)
                .expect("capability");
            grants & (1 << index) != 0
        }
        MethodCapability::None => true,
        MethodCapability::Unknown => false,
    }
}

fn check_diagnostics(method: &str, params: &Value) {
    if method != "editor.set_diagnostics" {
        return;
    }
    let Ok(params) = serde_json::from_value::<SetDiagnosticsParams>(params.clone()) else {
        return;
    };
    let valid = params.diagnostics.len() <= MAX_PLUGIN_DIAGNOSTICS
        && params.diagnostics.iter().all(|diagnostic| {
            diagnostic.message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES
                && diagnostic
                    .code
                    .as_ref()
                    .is_none_or(|code| code.len() <= MAX_DIAGNOSTIC_CODE_BYTES)
        });
    assert_eq!(validate_diagnostics(&params.diagnostics).is_ok(), valid);
}

fn check_sanitize(method: &str, params: &Value) {
    if method != "ui.message" {
        return;
    }
    let Some(message) = params.get("message").and_then(Value::as_str) else {
        return;
    };
    assert!(!sanitize(message).chars().any(char::is_control));
}

fn check_effect(method: &str, outcome: &Outcome, before: usize, after: usize) {
    if method == "editor.get_state" {
        assert_eq!(before, after);
        return;
    }
    if matches!(outcome, Outcome::Success) {
        assert!(after >= before);
    } else {
        assert_eq!(before, after);
    }
}
