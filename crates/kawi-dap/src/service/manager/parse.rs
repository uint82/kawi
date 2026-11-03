use super::*;
use serde_json::Value;
use std::collections::BTreeMap;

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
                named_variables: value.get("namedVariables").and_then(Value::as_i64),
                indexed_variables: value.get("indexedVariables").and_then(Value::as_i64),
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
                named_variables: value.get("namedVariables").and_then(Value::as_i64),
                indexed_variables: value.get("indexedVariables").and_then(Value::as_i64),
            })
        })
        .collect()
}

pub(super) fn threads(body: Option<&Value>) -> Vec<Thread> {
    array(body, "threads")
        .iter()
        .filter_map(|value| {
            Some(Thread {
                id: value.get("id").and_then(Value::as_i64)?,
                name: value
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("thread")
                    .to_string(),
            })
        })
        .collect()
}

pub(super) fn evaluate_result(body: Option<&Value>) -> (String, Option<String>, i64) {
    let result = body
        .and_then(|body| body.get("result"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let kind = body
        .and_then(|body| body.get("type"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let variables_reference = body
        .and_then(|body| body.get("variablesReference"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    (result, kind, variables_reference)
}

pub(super) fn set_variable_result(body: Option<&Value>) -> (String, Option<String>, i64) {
    let value = body
        .and_then(|body| body.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let kind = body
        .and_then(|body| body.get("type"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let variables_reference = body
        .and_then(|body| body.get("variablesReference"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    (value, kind, variables_reference)
}

pub(super) fn exception_filters(body: Option<&Value>) -> Vec<ExceptionFilter> {
    body.and_then(|body| body.get("exceptionBreakpointFilters"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|value| {
            Some(ExceptionFilter {
                filter: value.get("filter").and_then(Value::as_str)?.to_string(),
                label: value
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("filter")
                    .to_string(),
                default: value
                    .get("default")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

pub(super) fn run_in_terminal(request_seq: i64, arguments: Option<&Value>) -> RunInTerminalRequest {
    RunInTerminalRequest {
        request_seq,
        kind: string(arguments, "kind"),
        title: string(arguments, "title"),
        cwd: arguments
            .and_then(|arguments| arguments.get("cwd"))
            .and_then(Value::as_str)
            .map(PathBuf::from),
        args: string_array(arguments, "args"),
        env: string_map(arguments, "env"),
        args_can_be_interpreted_by_shell: arguments
            .and_then(|arguments| arguments.get("argsCanBeInterpretedByShell"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn string(value: Option<&Value>, key: &str) -> Option<String> {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn string_array(value: Option<&Value>, key: &str) -> Vec<String> {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn string_map(value: Option<&Value>, key: &str) -> BTreeMap<String, String> {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_object)
        .map(|values| {
            values
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
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
    use std::path::Path;

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
