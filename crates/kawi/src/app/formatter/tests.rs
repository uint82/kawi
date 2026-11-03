use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::*;
use crate::test_support::temp_dir;

fn shell(script: &str, stdin: bool) -> FormatterConfig {
    FormatterConfig {
        command: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            script.to_string(),
            "_".to_string(),
            PATH_PLACEHOLDER.to_string(),
        ],
        env: BTreeMap::new(),
        stdin,
    }
}

#[test]
fn stdin_formatter_returns_stdout() {
    let formatter = shell("tr a-z A-Z", true);
    let formatted = format(&formatter, Path::new("main.rs"), "fn main() {}\n");
    assert_eq!(formatted, Ok("FN MAIN() {}\n".to_string()));
}

#[test]
fn formatter_receives_environment() {
    let mut formatter = shell("printf '%s' \"$KAWI_FORMAT_TEST\"", true);
    formatter
        .env
        .insert("KAWI_FORMAT_TEST".to_string(), "value".to_string());
    let formatted = format(&formatter, Path::new("main.rs"), "");
    assert_eq!(formatted, Ok("value".to_string()));
}

#[test]
fn path_placeholder_is_expanded() {
    let formatter = FormatterConfig::command("cat").with_args(["--file", PATH_PLACEHOLDER]);
    assert_eq!(
        expanded_args(&formatter, Path::new("a b.rs")),
        vec!["--file".to_string(), "a b.rs".to_string()]
    );
}

#[test]
fn file_formatter_reads_rewritten_file() {
    let dir = temp_dir("formatter_file");
    let source = dir.join("main.rs");
    fs::write(&source, "old").unwrap();
    let formatter = shell("printf 'formatted' > \"$1\"", false);
    let formatted = format(&formatter, &source, "old");
    assert_eq!(formatted, Ok("formatted".to_string()));
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn formatter_failure_reports_stderr() {
    let formatter = shell("printf 'boom' >&2; exit 1", true);
    let error = format(&formatter, Path::new("main.rs"), "");
    assert_eq!(
        error,
        Err(FormatError::Failed("formatter failed: boom".to_string()))
    );
}

#[test]
fn missing_formatter_is_reported() {
    let formatter = FormatterConfig::command("definitely-missing-kawi-formatter");
    let error = format(&formatter, Path::new("main.rs"), "");
    assert!(matches!(
        error,
        Err(FormatError::NotFound(message))
            if message.contains("definitely-missing-kawi-formatter")
    ));
}
