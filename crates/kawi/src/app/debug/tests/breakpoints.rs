use super::*;

#[test]
fn breakpoint_options_are_reported_and_cleared() {
    let path = temp_file("breakpoint-options");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    assert!(app.debug.toggle_breakpoint(&app.editor));
    assert!(app
        .debug
        .set_breakpoint_condition(&app.editor, "x > 0".to_string()));
    assert!(app
        .debug
        .set_breakpoint_hit_condition(&app.editor, "3".to_string()));
    assert!(app
        .debug
        .set_breakpoint_log_message(&app.editor, "value {x}".to_string()));
    app.debug.show_breakpoints();
    let report = app.debug.take_report().expect("report");
    assert!(report
        .lines
        .iter()
        .any(|line| line.contains("condition: x > 0")));
    assert!(report.lines.iter().any(|line| line.contains("hit: 3")));
    assert!(report
        .lines
        .iter()
        .any(|line| line.contains("log: value {x}")));
    assert!(app.debug.clear_breakpoint_options(&app.editor));
    app.debug.show_breakpoints();
    let report = app.debug.take_report().expect("report");
    assert!(!report.lines.iter().any(|line| line.contains("condition:")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn function_and_exception_breakpoints_are_reported() {
    let path = temp_file("filters");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    assert!(app.debug.toggle_function_breakpoint("main".to_string()));
    app.debug.show_function_breakpoints();
    let report = app.debug.take_report().expect("report");
    assert!(report.lines.iter().any(|line| line == "main"));

    app.debug.apply_exception_filters(vec![
        ExceptionFilter {
            filter: "uncaught".to_string(),
            label: "Uncaught Exceptions".to_string(),
            default: true,
        },
        ExceptionFilter {
            filter: "caught".to_string(),
            label: "Caught Exceptions".to_string(),
            default: false,
        },
    ]);
    assert!(app.debug.enabled_exception_filters.contains("uncaught"));
    assert!(app.debug.toggle_exception_filter("caught".to_string()));
    app.debug.show_exception_filters();
    let report = app.debug.take_report().expect("report");
    assert!(report.lines.iter().any(|line| line.contains("[*] caught")));
    let _ = fs::remove_dir_all(root);
}
