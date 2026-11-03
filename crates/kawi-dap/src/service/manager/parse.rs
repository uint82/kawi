use super::*;
use serde_json::Value;

pub(super) fn breakpoints(body: Option<&Value>) -> Vec<Breakpoint> {
    array(body, "breakpoints").iter().map(breakpoint).collect()
}

pub(super) fn breakpoint(value: &Value) -> Breakpoint {
    Breakpoint {
        verified: value
            .get("verified")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        line: value
            .get("line")
            .and_then(Value::as_u64)
            .map(|line| line as u32),
        source: source_path(value),
        message: value
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

pub(super) fn stack_frames(body: Option<&Value>) -> Vec<StackFrame> {
    array(body, "stackFrames")
        .iter()
        .filter_map(|value| {
            Some(StackFrame {
                id: value.get("id").and_then(Value::as_i64)?,
                name: value
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("frame")
                    .to_string(),
                source: source_path(value),
                line: value.get("line").and_then(Value::as_u64).unwrap_or(1) as u32,
                column: value.get("column").and_then(Value::as_u64).unwrap_or(1) as u32,
            })
        })
        .collect()
}

pub(super) fn scopes(body: Option<&Value>) -> Vec<Scope> {
    array(body, "scopes")
        .iter()
        .filter_map(|value| {
            Some(Scope {
                name: value.get("name").and_then(Value::as_str)?.to_string(),
                variables_reference: value.get("variablesReference").and_then(Value::as_i64)?,
                expensive: value
                    .get("expensive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

pub(super) fn variables(body: Option<&Value>) -> Vec<Variable> {
    array(body, "variables")
        .iter()
        .filter_map(|value| {
            Some(Variable {
                name: value.get("name").and_then(Value::as_str)?.to_string(),
                value: value
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                kind: value
                    .get("type")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                variables_reference: value
                    .get("variablesReference")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
            })
        })
        .collect()
}

fn array<'a>(body: Option<&'a Value>, key: &str) -> &'a [Value] {
    body.and_then(|body| body.get(key))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn source_path(value: &Value) -> Option<PathBuf> {
    value
        .get("source")
        .and_then(|source| source.get("path"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_stack_frames_scopes_and_variables() {
        let frames = stack_frames(Some(&json!({
            "stackFrames": [{
                "id": 7,
                "name": "main",
                "source": {"path": "/tmp/main.rs"},
                "line": 3,
                "column": 5
            }]
        })));
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].source.as_deref(), Some(Path::new("/tmp/main.rs")));
        assert_eq!(frames[0].line, 3);

        let scopes = scopes(Some(&json!({
            "scopes": [{"name": "Locals", "variablesReference": 100, "expensive": false}]
        })));
        assert_eq!(scopes[0].variables_reference, 100);

        let variables = variables(Some(&json!({
            "variables": [{"name": "x", "value": "42", "type": "int", "variablesReference": 0}]
        })));
        assert_eq!(variables[0].name, "x");
        assert_eq!(variables[0].kind.as_deref(), Some("int"));
    }

    #[test]
    fn parses_breakpoints_with_optional_fields() {
        let breakpoints = breakpoints(Some(&json!({
            "breakpoints": [{"verified": true, "line": 4, "source": {"path": "/tmp/main.rs"}}]
        })));
        assert!(breakpoints[0].verified);
        assert_eq!(breakpoints[0].line, Some(4));
        assert_eq!(
            breakpoints[0].source.as_deref(),
            Some(Path::new("/tmp/main.rs"))
        );
    }

    #[test]
    fn missing_bodies_produce_empty_lists() {
        assert!(breakpoints(None).is_empty());
        assert!(stack_frames(None).is_empty());
        assert!(scopes(None).is_empty());
        assert!(variables(None).is_empty());
    }
}
