use super::*;

#[test]
fn evaluate_and_watch_events_update_reports() {
    let path = temp_file("evaluate");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    app.debug.apply_event(DapEvent::Evaluated {
        expression: "x + 1".to_string(),
        context: "repl".to_string(),
        result: "42".to_string(),
        kind: Some("int".to_string()),
        variables_reference: 0,
    });
    app.debug.show_console();
    let report = app.debug.take_report().expect("report");
    assert!(report.lines.iter().any(|line| line == "> x + 1"));
    assert!(report.lines.iter().any(|line| line.contains("42: int")));

    assert!(app.debug.add_watch("x".to_string()));
    app.debug.status = DebugStatus::Stopped;
    app.debug.apply_event(DapEvent::Evaluated {
        expression: "x".to_string(),
        context: "watch".to_string(),
        result: "7".to_string(),
        kind: Some("int".to_string()),
        variables_reference: 0,
    });
    app.debug.show_watches();
    let report = app.debug.take_report().expect("report");
    assert!(report
        .lines
        .iter()
        .any(|line| line.contains("1. x = 7: int")));
    assert!(app.debug.edit_watch(1, "y".to_string()));
    app.debug.apply_event(DapEvent::Evaluated {
        expression: "y".to_string(),
        context: "watch".to_string(),
        result: "8".to_string(),
        kind: Some("int".to_string()),
        variables_reference: 0,
    });
    app.debug.show_watches();
    let report = app.debug.take_report().expect("report");
    assert!(report
        .lines
        .iter()
        .any(|line| line.contains("1. y = 8: int")));
    assert!(app.debug.remove_watch("y"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn variable_scopes_and_expansion_are_reported() {
    let path = temp_file("variables");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    app.debug.current_frame = Some(StackFrame {
        id: 1,
        name: "main".to_string(),
        source: Some(path.clone()),
        line: 2,
        column: 1,
    });
    app.debug.pending_report = Some(ReportKind::Variables);
    app.debug.apply_scopes(
        1,
        vec![Scope {
            name: "Locals".to_string(),
            variables_reference: 100,
            expensive: false,
        }],
    );
    app.debug.apply_variables(
        100,
        vec![
            Variable {
                name: "x".to_string(),
                value: "42".to_string(),
                kind: Some("int".to_string()),
                variables_reference: 0,
            },
            Variable {
                name: "point".to_string(),
                value: "{...}".to_string(),
                kind: Some("Point".to_string()),
                variables_reference: 200,
            },
        ],
    );
    let report = app.debug.take_report().expect("report");
    assert!(report.lines.iter().any(|line| line.contains("Locals")));
    assert!(report.lines.iter().any(|line| line.contains("x: int = 42")));
    assert!(report.lines.iter().any(|line| line.contains("[ref 200]")));

    app.debug.pending_report = Some(ReportKind::Expand(200));
    app.debug.apply_variables(
        200,
        vec![Variable {
            name: "y".to_string(),
            value: "3".to_string(),
            kind: Some("int".to_string()),
            variables_reference: 0,
        }],
    );
    let report = app.debug.take_report().expect("report");
    assert!(report
        .lines
        .iter()
        .any(|line| line.contains("reference 200")));
    assert!(report.lines.iter().any(|line| line.contains("y: int = 3")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn threads_are_reported() {
    let path = temp_file("threads");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    app.debug.pending_report = Some(ReportKind::Threads);
    app.debug.thread_id = Some(2);
    app.debug.apply_threads(vec![
        Thread {
            id: 1,
            name: "main".to_string(),
        },
        Thread {
            id: 2,
            name: "worker".to_string(),
        },
    ]);
    let report = app.debug.take_report().expect("report");
    assert!(report
        .lines
        .iter()
        .any(|line| line.contains("[*] 2: worker")));
    let _ = fs::remove_dir_all(root);
}
