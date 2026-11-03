mod support;

use kawi_dap::{DapEvent, DapService, DebugStatus};
use std::fs;
use std::sync::mpsc;
use support::*;

#[test]
fn reports_capabilities_pages_variables_and_sets_values() {
    let dir = temp_dir("phase3_variables");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    assert!(service.start_request(start_request(config(&dir, true))));
    let capabilities = events.wait(&service, |event| {
        matches!(event, DapEvent::Capabilities { .. })
    });
    let DapEvent::Capabilities {
        supports_restart_request,
        supports_terminate_request,
    } = capabilities
    else {
        panic!("unexpected event");
    };
    assert!(supports_restart_request);
    assert!(supports_terminate_request);
    wait_for_stopped(&mut events, &service);

    assert!(service.variables_page(100, 0, 2));
    let first = events.wait(&service, |event| {
        matches!(event, DapEvent::Variables { start: 0, .. })
    });
    let DapEvent::Variables {
        variables, start, ..
    } = first
    else {
        panic!("unexpected event");
    };
    assert_eq!(start, 0);
    assert_eq!(variables.len(), 2);
    assert_eq!(variables[0].name, "x");
    assert_eq!(variables[1].name, "s");

    assert!(service.variables_page(100, 2, 2));
    let second = events.wait(&service, |event| {
        matches!(event, DapEvent::Variables { start: 2, .. })
    });
    let DapEvent::Variables {
        variables, start, ..
    } = second
    else {
        panic!("unexpected event");
    };
    assert_eq!(start, 2);
    assert_eq!(variables[0].name, "y");
    assert_eq!(variables[1].name, "z");

    assert!(service.set_variable(100, "x".to_string(), "99".to_string()));
    let set = events.wait(&service, |event| {
        matches!(event, DapEvent::VariableSet { .. })
    });
    let DapEvent::VariableSet {
        variables_reference,
        name,
        value,
        kind,
        new_reference,
    } = set
    else {
        panic!("unexpected event");
    };
    assert_eq!(variables_reference, 100);
    assert_eq!(name, "x");
    assert_eq!(value, "99");
    assert_eq!(kind.as_deref(), Some("string"));
    assert_eq!(new_reference, 0);

    assert!(service.stop());
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn restarts_and_terminates_through_the_adapter() {
    let dir = temp_dir("phase3_lifecycle");
    let request_log = dir.join("requests.log");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    let mut configuration = config(&dir, true);
    configuration.env.insert(
        "FAKE_DAP_REQUEST_LOG".to_string(),
        request_log.display().to_string(),
    );
    assert!(service.start_request(start_request(configuration)));
    wait_for_stopped(&mut events, &service);

    assert!(service.restart());
    events.wait(
        &service,
        |event| matches!(event, DapEvent::Log(message) if message.contains("restart")),
    );
    let stopped = events.wait(
        &service,
        |event| matches!(event, DapEvent::Stopped { reason, .. } if reason == "restart"),
    );
    assert!(matches!(stopped, DapEvent::Stopped { .. }));

    assert!(service.terminate());
    events.wait(
        &service,
        |event| matches!(event, DapEvent::Log(message) if message.contains("terminate")),
    );
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    let log = fs::read_to_string(&request_log).unwrap();
    assert!(log.lines().any(|line| line == "restart"));
    assert!(log.lines().any(|line| line == "terminate"));
    let _ = fs::remove_dir_all(dir);
}
