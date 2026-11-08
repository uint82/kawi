use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn loaded_panel_app(name: &str) -> (App, PathBuf) {
    let path = temp_file(name);
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    app.debug.show_panel();
    app.debug.status = DebugStatus::Stopped;
    app.debug.thread_id = Some(1);
    app.debug.session.threads = vec![Thread {
        id: 1,
        name: "main".to_string(),
    }];
    app.debug.stack = vec![StackFrame {
        id: 1,
        name: "main".to_string(),
        source: Some(path.clone()),
        line: 2,
        column: 1,
    }];
    app.debug.current_frame = app.debug.stack.first().cloned();
    app.debug.scopes = vec![Scope {
        name: "Locals".to_string(),
        variables_reference: 100,
        named_variables: Some(3),
        indexed_variables: Some(0),
        expensive: false,
    }];
    app.debug.variables.insert(
        100,
        vec![
            Variable {
                name: "x".to_string(),
                value: "42".to_string(),
                kind: Some("int".to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            },
            Variable {
                name: "s".to_string(),
                value: "hello".to_string(),
                kind: Some("str".to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            },
        ],
    );
    app.debug.variable_totals.insert(100, 3);
    app.debug.expanded_variables.insert(100);
    app.debug.watches.push(WatchExpression {
        expression: "x".to_string(),
        result: Some("42".to_string()),
        kind: Some("int".to_string()),
        variables_reference: 0,
    });
    app.debug.breakpoints.insert(
        path.clone(),
        BTreeMap::from([(
            2,
            BreakpointSpec {
                condition: Some("x > 0".to_string()),
                ..BreakpointSpec::default()
            },
        )]),
    );
    app.debug.function_breakpoints.insert("main".to_string());
    app.debug.apply_exception_filters(vec![ExceptionFilter {
        filter: "uncaught".to_string(),
        label: "Uncaught Exceptions".to_string(),
        default: true,
    }]);
    app.debug.console.push_back(ConsoleEntry {
        input: "x".to_string(),
        output: "42".to_string(),
        kind: Some("int".to_string()),
        variables_reference: 0,
        is_error: false,
    });
    (app, root)
}

#[test]
fn q_closes_the_debug_panel() {
    let (mut app, root) = loaded_panel_app("panel-close-q");
    app.debug_focus = true;
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .expect("key");
    assert!(!app.debug.panel_visible());
    assert!(!app.debug_focus);
    assert!(app.is_editor_focused());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn escape_returns_focus_and_keeps_the_debug_panel() {
    let (mut app, root) = loaded_panel_app("panel-close-esc");
    app.debug_focus = true;
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .expect("key");
    assert!(app.debug.panel_visible());
    assert!(!app.debug_focus);
    assert!(app.is_editor_focused());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn panel_entries_render_the_session_tree() {
    let (app, root) = loaded_panel_app("panel-rows");
    let entries = app.debug.panel_entries();
    let labels: Vec<&str> = entries.iter().map(|entry| entry.label.as_str()).collect();
    assert!(labels.contains(&"Threads"));
    assert!(labels.contains(&"main"));
    assert!(labels.contains(&"Locals"));
    assert!(labels.contains(&"x: int = 42"));
    assert!(labels.contains(&"Load more (2/3)"));
    assert!(labels.contains(&"Watches"));
    assert!(labels.contains(&"1. x = 42: int"));
    assert!(labels.contains(&"Function Breakpoints"));
    assert!(labels.contains(&"Exception Filters"));
    assert!(labels.contains(&"Console"));
    assert!(app.debug.panel_width(80) > 0);
    assert_eq!(app.debug.panel_width(30), 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn panel_activation_toggles_variables_and_breakpoints() {
    let (mut app, root) = loaded_panel_app("panel-actions");
    let entries = app.debug.panel_entries();
    let scope = entries
        .iter()
        .position(|entry| entry.label == "Locals")
        .expect("scope row");
    app.debug.panel_selected = scope;
    assert!(app.debug.activate_panel());
    assert!(!app.debug.expanded_variables.contains(&100));
    assert!(app.debug.activate_panel());
    assert!(app.debug.expanded_variables.contains(&100));

    let breakpoint = app
        .debug
        .panel_entries()
        .iter()
        .position(|entry| entry.label.contains("main.rs:2 if x > 0"))
        .expect("breakpoint row");
    app.debug.panel_selected = breakpoint;
    assert!(app.debug.activate_panel());
    assert_eq!(app.debug.take_jump().map(|jump| jump.line), Some(1));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn paging_appends_variables_and_set_updates_values() {
    let path = temp_file("panel-paging");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    app.debug.status = DebugStatus::Stopped;
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
            named_variables: Some(3),
            indexed_variables: Some(0),
            expensive: false,
        }],
    );
    app.debug.apply_variables(
        100,
        vec![Variable {
            name: "x".to_string(),
            value: "1".to_string(),
            kind: Some("int".to_string()),
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
        }],
        0,
    );
    app.debug.apply_variables(
        100,
        vec![Variable {
            name: "y".to_string(),
            value: "2".to_string(),
            kind: Some("int".to_string()),
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
        }],
        1,
    );
    assert_eq!(app.debug.variables[&100].len(), 2);
    assert_eq!(app.debug.variable_totals[&100], 3);
    app.debug.apply_variable_set(
        100,
        "x".to_string(),
        "9".to_string(),
        Some("int".to_string()),
        0,
    );
    assert_eq!(app.debug.variables[&100][0].value, "9");
    let _ = fs::remove_dir_all(root);
}
