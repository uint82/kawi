use super::*;

pub(super) fn expand_json(value: Value, context: &DebugContext) -> Result<Value, String> {
    match value {
        Value::String(text) => Ok(Value::String(expand_string(&text, context)?)),
        Value::Array(items) => items
            .into_iter()
            .map(|item| expand_json(item, context))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(items) => items
            .into_iter()
            .map(|(key, value)| Ok((key, expand_json(value, context)?)))
            .collect::<Result<Map<_, _>, String>>()
            .map(Value::Object),
        value => Ok(value),
    }
}

pub(super) fn expand_path(path: &Path, context: &DebugContext) -> Result<PathBuf, String> {
    let expanded = expand_string(&path.to_string_lossy(), context)?;
    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(context.workspace.join(path))
    }
}

pub(super) fn expand_string(input: &str, context: &DebugContext) -> Result<String, String> {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            return Err(format!("unterminated variable in {input}"));
        };
        output.push_str(&variable(&after[..end], context)?);
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    Ok(output)
}

fn variable(name: &str, context: &DebugContext) -> Result<String, String> {
    match name {
        "workspace" => Ok(context.workspace.display().to_string()),
        "file" => context
            .file
            .as_ref()
            .map(|path| path.display().to_string())
            .ok_or_else(|| "debug variable ${file} requires a file buffer".to_string()),
        "fileDirname" => context
            .file
            .as_ref()
            .and_then(|path| path.parent())
            .map(|path| path.display().to_string())
            .ok_or_else(|| "debug variable ${fileDirname} requires a file buffer".to_string()),
        _ if name.starts_with("env:") => {
            std::env::var(&name[4..]).map_err(|_| format!("environment variable {name} is not set"))
        }
        _ => Err(format!("unknown debug variable ${{{name}}}")),
    }
}

pub(super) fn toml_to_json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(value) => Value::String(value),
        toml::Value::Integer(value) => Value::from(value),
        toml::Value::Float(value) => Value::from(value),
        toml::Value::Boolean(value) => Value::Bool(value),
        toml::Value::Datetime(value) => Value::String(value.to_string()),
        toml::Value::Array(values) => Value::Array(values.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, toml_to_json(value)))
                .collect(),
        ),
    }
}
