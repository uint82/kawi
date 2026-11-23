use super::super::ApiSignatureMetadata;
use super::{optional, required, signature};

pub(super) fn lookup(name: &str) -> Option<ApiSignatureMetadata> {
    Some(match name {
        "autocmd.clear" => signature(parameters![], "null"),
        "autocmd.list" => signature(parameters![], "RegisteredAutocmd[]"),
        "autocmd.register" => signature(
            parameters![
                required("event", "string"),
                optional("pattern", "string"),
                optional("buffer", "integer"),
                optional("once", "boolean"),
            ],
            "RegisteredAutocmd",
        ),
        "autocmd.remove" => signature(parameters![required("id", "integer")], "null"),
        "config.get" => signature(parameters![required("key", "string")], "ConfigGetResult"),
        "config.list" => signature(parameters![], "ConfigListResult"),
        "process.close_stdin" => signature(parameters![required("job", "integer")], "null"),
        "process.kill" => signature(parameters![required("job", "integer")], "null"),
        "process.open" => signature(
            parameters![
                required("command", "string"),
                optional("args", "string[]"),
                optional("cwd", "string"),
                optional("env", "object"),
                optional("stdin", "string"),
            ],
            "ProcessOpenResult",
        ),
        "process.poll" => signature(parameters![required("job", "integer")], "ProcessJobEvent[]"),
        "process.signal" => signature(
            parameters![required("job", "integer"), required("signal", "string"),],
            "null",
        ),
        "process.status" => signature(
            parameters![required("job", "integer")],
            "ProcessStatusResult",
        ),
        "process.write" => signature(
            parameters![required("job", "integer"), required("data", "string"),],
            "null",
        ),
        "task.cancel" => signature(parameters![required("job", "integer")], "null"),
        "task.list" => signature(parameters![], "TaskInfo[]"),
        "task.poll" => signature(parameters![required("job", "integer")], "TaskEvent[]"),
        "task.start" => signature(parameters![required("name", "string")], "TaskStartResult"),
        "task.status" => signature(parameters![required("job", "integer")], "TaskStatusResult"),
        "timer.after" => signature(
            parameters![
                required("delay_ms", "integer"),
                optional("payload", "string"),
            ],
            "integer",
        ),
        "timer.cancel" => signature(parameters![required("timer", "integer")], "boolean"),
        "timer.every" => signature(
            parameters![
                required("interval_ms", "integer"),
                optional("payload", "string"),
            ],
            "integer",
        ),
        "timer.list" => signature(parameters![], "TimerInfo[]"),
        "variable.delete" => signature(
            parameters![
                required("target", "VariableTarget"),
                required("key", "string"),
            ],
            "boolean",
        ),
        "variable.get" => signature(
            parameters![
                required("target", "VariableTarget"),
                required("key", "string"),
            ],
            "any",
        ),
        "variable.list" => signature(
            parameters![required("target", "VariableTarget")],
            "Variable[]",
        ),
        "variable.set" => signature(
            parameters![
                required("target", "VariableTarget"),
                required("key", "string"),
                required("value", "any"),
            ],
            "null",
        ),
        _ => return None,
    })
}
