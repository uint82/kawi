mod support;

use kawi_dap::{BreakpointSet, DapService, DebugStatus, SourceBreakpoint};
use std::fs;
use std::sync::mpsc;
use support::*;

#[test]
fn sends_conditional_breakpoint_fields() {
    let dir = temp_dir("conditional");
    let request_log = dir.join("requests.log");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    let mut configuration = config(&dir, false);
    configuration.env.insert(
        "FAKE_DAP_REQUEST_LOG".to_string(),
        request_log.display().to_string(),
    );
    let mut request = start_request(configuration);
    request.breakpoints = vec![BreakpointSet {
        path: dir.join("main.rs"),
        breakpoints: vec![SourceBreakpoint {
            line: 4,
            condition: Some("x > 0".to_string()),
            hit_condition: Some("3".to_string()),
            log_message: Some("value {x}".to_string()),
        }],
    }];
    assert!(service.start_request(request));
    wait_for_status(&mut events, &service, DebugStatus::Running);
    let log = fs::read_to_string(&request_log).unwrap();
    let line = log
        .lines()
        .find(|line| line.starts_with("setBreakpoints"))
        .expect("setBreakpoints request");
    assert!(line.contains("\"condition\":\"x > 0\""));
    assert!(line.contains("\"hitCondition\":\"3\""));
    assert!(line.contains("\"logMessage\":\"value {x}\""));
    assert!(service.stop());
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    let _ = fs::remove_dir_all(dir);
}
