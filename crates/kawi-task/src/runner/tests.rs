use super::reader::{sanitize_line, MAX_OUTPUT_LINE_CHARS};
use super::*;

use crate::parser::{OutputStream, ParserSpec};
use std::collections::BTreeMap;

fn spec(command: &str, args: &[&str], parser: ParserSpec) -> TaskSpec {
    TaskSpec {
        name: "test".to_string(),
        command: command.to_string(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
        cwd: None,
        env: BTreeMap::new(),
        parser,
    }
}

fn wait(runner: &TaskRunner) -> Vec<TaskEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    while Instant::now() < deadline {
        while let Some(event) = runner.try_recv() {
            let finished = matches!(event, TaskEvent::Finished { .. });
            events.push(event);
            if finished {
                return events;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("task did not finish");
}

#[test]
fn strips_terminal_sequences_and_bounds_lines() {
    let text = format!("\u{1b}[31merror\u{1b}[0m\tvalue\r{}", "x".repeat(5_000));
    let line = sanitize_line(&text);
    assert!(line.starts_with("error    value"));
    assert!(!line.contains('\u{1b}'));
    assert_eq!(line.chars().count(), MAX_OUTPUT_LINE_CHARS);
}

#[test]
#[cfg(unix)]
fn streams_stdout_and_stderr() {
    let root = std::env::temp_dir();
    let wake = mpsc::channel().0;
    let runner = TaskRunner::start(
        &spec(
            "sh",
            &["-c", "printf 'one\\n'; printf 'two\\n' >&2"],
            ParserSpec::None,
        ),
        &root,
        wake,
    )
    .unwrap();
    let events = wait(&runner);
    let lines: Vec<(OutputStream, String)> = events
        .iter()
        .filter_map(|event| match event {
            TaskEvent::Output { stream, line } => Some((*stream, line.clone())),
            _ => None,
        })
        .collect();
    assert!(lines.contains(&(OutputStream::Stdout, "one".to_string())));
    assert!(lines.contains(&(OutputStream::Stderr, "two".to_string())));
    assert!(matches!(
        events.last(),
        Some(TaskEvent::Finished {
            status: TaskStatus { success: true, .. },
            ..
        })
    ));
}

#[test]
#[cfg(unix)]
fn reports_findings_and_failure() {
    let root = std::env::temp_dir();
    let wake = mpsc::channel().0;
    let runner = TaskRunner::start(
        &spec(
            "sh",
            &["-c", "printf 'a.go:2:3: bad\\n'; exit 1"],
            ParserSpec::Go,
        ),
        &root,
        wake,
    )
    .unwrap();
    let events = wait(&runner);
    let finding = events
        .iter()
        .find_map(|event| match event {
            TaskEvent::Finding(finding) => Some(finding),
            _ => None,
        })
        .unwrap();
    assert_eq!(finding.path, root.join("a.go"));
    assert_eq!(finding.line, 2);
    assert!(matches!(
        events.last(),
        Some(TaskEvent::Finished {
            status: TaskStatus { success: false, .. },
            description,
        }) if description == "exit 1"
    ));
}

#[test]
#[cfg(unix)]
fn cancels_the_whole_process_group() {
    let root = std::env::temp_dir();
    let wake = mpsc::channel().0;
    let runner = TaskRunner::start(
        &spec("sh", &["-c", "sleep 10 & wait"], ParserSpec::None),
        &root,
        wake,
    )
    .unwrap();
    runner.cancel();
    let events = wait(&runner);
    assert!(matches!(
        events.last(),
        Some(TaskEvent::Finished {
            status: TaskStatus {
                cancelled: true,
                success: false,
                ..
            },
            ..
        })
    ));
}

#[test]
#[cfg(unix)]
fn detached_task_has_no_controlling_terminal() {
    let root = std::env::temp_dir();
    let wake = mpsc::channel().0;
    let runner = TaskRunner::start_with(
        &spec(
            "sh",
            &[
                "-c",
                "if (exec 3</dev/tty) 2>/dev/null; then exit 1; fi; case \"$(ps -o stat= -p $$)\" in *s*) ;; *) exit 1;; esac",
            ],
            ParserSpec::None,
        ),
        &root,
        wake,
        TaskOptions { detached: true },
    )
    .unwrap();
    let events = wait(&runner);
    assert!(matches!(
        events.last(),
        Some(TaskEvent::Finished {
            status: TaskStatus { success: true, .. },
            ..
        })
    ));
}
