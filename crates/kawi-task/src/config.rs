use crate::parser::{infer_parser, parser_from_name, ParserSpec, RegexParser};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, TableLike};

const TASK_FIELDS: &[&str] = &[
    "args", "command", "cwd", "env", "parser", "pattern", "severity",
];

#[derive(Debug, Clone)]
pub struct TaskSpec {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub parser: ParserSpec,
}

#[derive(Debug, Default)]
pub struct Tasks {
    tasks: BTreeMap<String, TaskSpec>,
}

impl Tasks {
    pub fn get(&self, name: &str) -> Option<&TaskSpec> {
        self.tasks.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.tasks.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct TaskLoad {
    pub tasks: Tasks,
    pub warnings: Vec<TaskWarning>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskWarning {
    line: Option<usize>,
    message: String,
}

impl TaskWarning {
    pub fn format(&self, path: &Path) -> String {
        let location = match self.line {
            Some(line) => format!("{}:{line}", path.display()),
            None => path.display().to_string(),
        };
        format!("{location}: {}", self.message)
    }
}

pub fn load(path: &Path) -> TaskLoad {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return TaskLoad::default(),
        Err(error) => {
            return TaskLoad {
                tasks: Tasks::default(),
                warnings: vec![TaskWarning {
                    line: None,
                    message: format!("tasks: rejected {error}: using no tasks"),
                }],
            }
        }
    };
    match text.parse::<DocumentMut>() {
        Ok(document) => load_document(&text, &document),
        Err(error) => TaskLoad {
            tasks: Tasks::default(),
            warnings: vec![TaskWarning {
                line: line_at(&text, error.span()),
                message: format!("tasks: rejected {}: using no tasks", error.message()),
            }],
        },
    }
}

fn load_document(text: &str, document: &DocumentMut) -> TaskLoad {
    let mut tasks = Tasks::default();
    let mut warnings = Vec::new();
    for (key, item) in document.iter() {
        if key != "task" {
            warnings.push(warning(item, text, format!("{key}: ignored")));
            continue;
        }
        let Some(table) = item.as_table_like() else {
            warnings.push(warning(
                item,
                text,
                "task: rejected value: expected a table of tasks",
            ));
            continue;
        };
        load_tasks(text, table, &mut tasks, &mut warnings);
    }
    TaskLoad { tasks, warnings }
}

fn load_tasks(
    text: &str,
    table: &dyn TableLike,
    tasks: &mut Tasks,
    warnings: &mut Vec<TaskWarning>,
) {
    for (name, item) in table.iter() {
        let name = name.trim();
        if name.is_empty() {
            warnings.push(warning(item, text, "task: rejected empty name"));
            continue;
        }
        let Some(task_table) = item.as_table_like() else {
            warnings.push(warning(
                item,
                text,
                format!("task {name}: rejected value: expected a table"),
            ));
            continue;
        };
        let Some(task) = load_task(text, name, task_table, warnings) else {
            continue;
        };
        if tasks.tasks.contains_key(name) {
            warnings.push(warning(
                item,
                text,
                format!("task {name}: rejected duplicate task"),
            ));
            continue;
        }
        tasks.tasks.insert(name.to_string(), task);
    }
}

fn load_task(
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) -> Option<TaskSpec> {
    warn_unknown_fields(text, name, table, warnings);
    let command = read_command(text, name, table, warnings)?;
    let args = read_args(text, name, table, warnings);
    let cwd = read_cwd(text, name, table, warnings);
    let env = read_env(text, name, table, warnings);
    let parser = read_parser(&command, text, name, table, warnings);
    Some(TaskSpec {
        name: name.to_string(),
        command,
        args,
        cwd,
        env,
        parser,
    })
}

fn read_command(
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) -> Option<String> {
    let Some(item) = table.get("command") else {
        warnings.push(unlocated(format!("task {name}: rejected missing command")));
        return None;
    };
    let Some(command) = item
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        warnings.push(warning(
            item,
            text,
            format!("task {name}: rejected command: expected a non-empty string"),
        ));
        return None;
    };
    Some(command.to_string())
}

fn read_args(
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) -> Vec<String> {
    let Some(item) = table.get("args") else {
        return Vec::new();
    };
    let Some(array) = item.as_array() else {
        warnings.push(warning(
            item,
            text,
            format!("task {name}: rejected args: expected an array of strings"),
        ));
        return Vec::new();
    };
    let mut args = Vec::new();
    for value in array.iter() {
        match value.as_str() {
            Some(value) => args.push(value.to_string()),
            None => warnings.push(warning(
                item,
                text,
                format!("task {name}: rejected args: expected an array of strings"),
            )),
        }
    }
    args
}

fn read_cwd(
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) -> Option<PathBuf> {
    let item = table.get("cwd")?;
    let Some(value) = item
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        warnings.push(warning(
            item,
            text,
            format!("task {name}: rejected cwd: expected a non-empty string"),
        ));
        return None;
    };
    Some(PathBuf::from(value))
}

fn read_env(
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) -> BTreeMap<String, String> {
    let Some(item) = table.get("env") else {
        return BTreeMap::new();
    };
    let Some(env) = item.as_table_like() else {
        warnings.push(warning(
            item,
            text,
            format!("task {name}: rejected env: expected a table of strings"),
        ));
        return BTreeMap::new();
    };
    let mut values = BTreeMap::new();
    for (key, item) in env.iter() {
        match item.as_str() {
            Some(value) => {
                values.insert(key.to_string(), value.to_string());
            }
            None => warnings.push(warning(
                item,
                text,
                format!("task {name}: rejected env.{key}: expected a string"),
            )),
        }
    }
    values
}

fn read_parser(
    command: &str,
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) -> ParserSpec {
    if let Some(pattern) = table.get("pattern") {
        return custom_parser(pattern, text, name, table, warnings);
    }
    let Some(item) = table.get("parser") else {
        return infer_parser(command);
    };
    let Some(name_value) = item.as_str() else {
        warnings.push(warning(
            item,
            text,
            format!("task {name}: rejected parser: expected a name"),
        ));
        return infer_parser(command);
    };
    match parser_from_name(name_value) {
        Some(parser) => parser,
        None => {
            warnings.push(warning(
                item,
                text,
                format!(
                    "task {name}: rejected parser: unknown parser {name_value}, using command detection"
                ),
            ));
            infer_parser(command)
        }
    }
}

fn custom_parser(
    pattern_item: &Item,
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) -> ParserSpec {
    let Some(pattern) = pattern_item.as_str() else {
        warnings.push(warning(
            pattern_item,
            text,
            format!("task {name}: rejected pattern: expected a string"),
        ));
        return ParserSpec::None;
    };
    let severity = read_severity(table, text, name, warnings);
    let severity = severity.as_str();
    if table.get("parser").is_some() {
        warnings.push(warning(
            pattern_item,
            text,
            format!("task {name}: parser ignored because pattern is set"),
        ));
    }
    match RegexParser::new(pattern, severity) {
        Ok(parser) => ParserSpec::Regex(parser),
        Err(error) => {
            warnings.push(warning(
                pattern_item,
                text,
                format!("task {name}: rejected pattern: {error}"),
            ));
            ParserSpec::None
        }
    }
}

fn read_severity(
    table: &dyn TableLike,
    text: &str,
    name: &str,
    warnings: &mut Vec<TaskWarning>,
) -> String {
    let Some(item) = table.get("severity") else {
        return "error".to_string();
    };
    let Some(severity) = item.as_str() else {
        warnings.push(warning(
            item,
            text,
            format!("task {name}: rejected severity: expected a name"),
        ));
        return "error".to_string();
    };
    if crate::parser::Severity::from_name(severity).is_none() {
        warnings.push(warning(
            item,
            text,
            format!("task {name}: rejected severity: unknown severity {severity}"),
        ));
        return "error".to_string();
    }
    severity.to_string()
}

fn warn_unknown_fields(
    text: &str,
    name: &str,
    table: &dyn TableLike,
    warnings: &mut Vec<TaskWarning>,
) {
    for (key, item) in table.iter() {
        if TASK_FIELDS.contains(&key) {
            continue;
        }
        warnings.push(warning(item, text, format!("task {name}.{key}: ignored")));
    }
}

fn warning(item: &Item, text: &str, message: impl Into<String>) -> TaskWarning {
    TaskWarning {
        line: line_at(text, item.span()),
        message: message.into(),
    }
}

fn unlocated(message: impl Into<String>) -> TaskWarning {
    TaskWarning {
        line: None,
        message: message.into(),
    }
}

fn line_at(text: &str, span: Option<std::ops::Range<usize>>) -> Option<usize> {
    span.map(|span| {
        text[..span.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ParserSpec;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_file(name: &str, text: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let dir = std::env::temp_dir().join(format!(
            "kawi_task_config_{}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tasks.toml");
        fs::write(&path, text).unwrap();
        path
    }

    fn load_text(name: &str, text: &str) -> TaskLoad {
        let path = temp_file(name, text);
        let loaded = load(&path);
        let _ = fs::remove_dir_all(path.parent().unwrap());
        loaded
    }

    #[test]
    fn loads_tasks_with_fields_and_parser_inference() {
        let loaded = load_text(
            "load",
            r#"
[task.check]
command = "cargo"
args = ["check", "--message-format=json"]
cwd = "crates/kawi"
env = { RUSTFLAGS = "-D warnings" }

[task.c]
command = "cc"
args = ["-c", "main.c"]

[task.custom]
command = "custom"
pattern = '^(?P<file>[^:]+):(?P<line>\d+): (?P<message>.+)$'
severity = "warning"
"#,
        );
        assert!(loaded.warnings.is_empty());
        assert_eq!(
            loaded.tasks.names().collect::<Vec<_>>(),
            ["c", "check", "custom"]
        );
        let check = loaded.tasks.get("check").unwrap();
        assert_eq!(check.args, ["check", "--message-format=json"]);
        assert_eq!(check.cwd, Some(PathBuf::from("crates/kawi")));
        assert_eq!(check.env.get("RUSTFLAGS").unwrap(), "-D warnings");
        assert!(matches!(check.parser, ParserSpec::Cargo));
        assert!(matches!(
            loaded.tasks.get("c").unwrap().parser,
            ParserSpec::Gcc
        ));
        assert!(matches!(
            loaded.tasks.get("custom").unwrap().parser,
            ParserSpec::Regex(_)
        ));
    }

    #[test]
    fn missing_file_has_no_tasks_or_warnings() {
        let loaded = load(Path::new("/tmp/kawi-task-definitely-missing/tasks.toml"));
        assert!(loaded.tasks.is_empty());
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn reports_invalid_fields_and_parser_settings() {
        let loaded = load_text(
            "warnings",
            r#"
[other]
value = 1

[task.missing]
args = ["x"]

[task.odd]
command = "true"
parser = "gcc"
pattern = "("
severity = "loud"
unknown = true
"#,
        );
        let messages = loaded
            .warnings
            .iter()
            .map(|warning| warning.message.as_str())
            .collect::<Vec<_>>();
        assert!(messages
            .iter()
            .any(|message| message.contains("other: ignored")));
        assert!(messages
            .iter()
            .any(|message| message.contains("task missing: rejected missing command")));
        assert!(messages
            .iter()
            .any(|message| message.contains("parser ignored because pattern is set")));
        assert!(messages
            .iter()
            .any(|message| message.contains("unknown severity loud")));
        assert!(messages
            .iter()
            .any(|message| message.contains("rejected pattern")));
        assert!(messages
            .iter()
            .any(|message| message.contains("task odd.unknown: ignored")));
        assert!(matches!(
            loaded.tasks.get("odd").unwrap().parser,
            ParserSpec::None
        ));
        assert!(loaded.tasks.get("missing").is_none());
    }

    #[test]
    fn malformed_file_reports_a_line() {
        let loaded = load_text("malformed", "[task.check\ncommand = \"cargo\"\n");
        assert_eq!(loaded.warnings.len(), 1);
        assert_eq!(loaded.warnings[0].line, Some(1));
        assert!(loaded.tasks.is_empty());
    }

    #[test]
    fn warning_format_includes_path_and_line() {
        let warning = TaskWarning {
            line: Some(4),
            message: "task check: rejected args".to_string(),
        };
        assert_eq!(
            warning.format(Path::new("tasks.toml")),
            "tasks.toml:4: task check: rejected args"
        );
    }
}
