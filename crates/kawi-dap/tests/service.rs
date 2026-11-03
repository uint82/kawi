use kawi_dap::{
    BreakpointSet, DapEvent, DapService, DebugStatus, ExpandedConfiguration, FunctionBreakpoint,
    SourceBreakpoint, StartRequest, StepKind,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const FAKE_SERVER: &str = env!("CARGO_BIN_EXE_fake-dap-server");

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kawi_dap_service_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn config(dir: &Path, stop_on_entry: bool) -> ExpandedConfiguration {
    let mut env = BTreeMap::new();
    env.insert(
        "FAKE_DAP_SOURCE".to_string(),
        dir.join("main.rs").display().to_string(),
    );
    env.insert(
        "FAKE_DAP_PID_FILE".to_string(),
        dir.join("server.pid").display().to_string(),
    );
    env.insert(
        "FAKE_DAP_STOP_ON_ENTRY".to_string(),
        if stop_on_entry { "1" } else { "0" }.to_string(),
    );
    ExpandedConfiguration {
        name: "test".to_string(),
        adapter: "fake".to_string(),
        request: "launch".to_string(),
        command: FAKE_SERVER.to_string(),
        args: Vec::new(),
        env,
        cwd: Some(dir.to_path_buf()),
        arguments: json!({"program": dir.join("main.rs").display().to_string()}),
    }
}

fn start_request(config: ExpandedConfiguration) -> StartRequest {
    StartRequest {
        config,
        breakpoints: Vec::new(),
        function_breakpoints: Vec::new(),
        exception_filters: None,
    }
}

fn breakpoint_set(dir: &Path, lines: &[u32]) -> BreakpointSet {
    BreakpointSet {
        path: dir.join("main.rs"),
        breakpoints: lines
            .iter()
            .map(|line| SourceBreakpoint {
                line: *line,
                condition: None,
                hit_condition: None,
                log_message: None,
            })
            .collect(),
    }
}

struct Events {
    seen: Vec<DapEvent>,
}

impl Events {
    fn new() -> Self {
        Self { seen: Vec::new() }
    }

    fn wait(&mut self, service: &DapService, predicate: impl Fn(&DapEvent) -> bool) -> DapEvent {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            while let Some(event) = service.try_recv() {
                self.seen.push(event);
            }
            if let Some(index) = self.seen.iter().position(&predicate) {
                return self.seen.remove(index);
            }
            if Instant::now() >= deadline {
                panic!("event timed out: {:?}", self.seen);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn wait_for_status(events: &mut Events, service: &DapService, status: DebugStatus) {
    events.wait(
        service,
        |event| matches!(event, DapEvent::Status(current) if *current == status),
    );
}

fn wait_for_stopped(events: &mut Events, service: &DapService) -> DapEvent {
    events.wait(service, |event| matches!(event, DapEvent::Stopped { .. }))
}

#[test]
fn runs_a_session_through_launch_breakpoints_and_variables() {
    let dir = temp_dir("session");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    let mut request = start_request(config(&dir, true));
    request.breakpoints = vec![breakpoint_set(&dir, &[1])];
    assert!(service.start_request(request));
    wait_for_status(&mut events, &service, DebugStatus::Running);
    let breakpoint = events.wait(&service, |event| {
        matches!(event, DapEvent::Breakpoint { .. })
    });
    let DapEvent::Breakpoint { breakpoint, .. } = breakpoint else {
        panic!("unexpected event");
    };
    assert!(breakpoint.verified);
    assert_eq!(breakpoint.line, Some(1));
    let stopped = wait_for_stopped(&mut events, &service);
    let DapEvent::Stopped {
        reason, thread_id, ..
    } = stopped
    else {
        panic!("unexpected event");
    };
    assert_eq!(reason, "entry");
    assert_eq!(thread_id, Some(1));

    assert!(service.continue_thread(1));
    let stopped = events.wait(
        &service,
        |event| matches!(event, DapEvent::Stopped { reason, .. } if reason == "breakpoint"),
    );
    let DapEvent::Stopped { thread_id, .. } = stopped else {
        panic!("unexpected event");
    };
    assert_eq!(thread_id, Some(1));

    assert!(service.stack_trace(1));
    let stack = events.wait(&service, |event| {
        matches!(event, DapEvent::StackTrace { .. })
    });
    let DapEvent::StackTrace { frames, .. } = stack else {
        panic!("unexpected event");
    };
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].name, "main");
    assert_eq!(frames[0].line, 2);

    assert!(service.scopes(1));
    let scopes = events.wait(&service, |event| matches!(event, DapEvent::Scopes { .. }));
    let DapEvent::Scopes { scopes, .. } = scopes else {
        panic!("unexpected event");
    };
    assert_eq!(scopes[0].name, "Locals");

    assert!(service.variables(100));
    let variables = events.wait(&service, |event| {
        matches!(event, DapEvent::Variables { .. })
    });
    let DapEvent::Variables { variables, .. } = variables else {
        panic!("unexpected event");
    };
    assert!(variables
        .iter()
        .any(|variable| variable.name == "x" && variable.value == "42"));

    assert!(service.stop());
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    assert!(dir.join("server.pid").is_file());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn updates_breakpoints_after_initialization() {
    let dir = temp_dir("breakpoints");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    assert!(service.start_request(start_request(config(&dir, true))));
    wait_for_stopped(&mut events, &service);
    assert!(service.set_breakpoints(
        dir.join("main.rs"),
        vec![SourceBreakpoint {
            line: 4,
            condition: Some("x > 0".to_string()),
            hit_condition: None,
            log_message: None,
        }],
    ));
    let event = events.wait(&service, |event| {
        matches!(event, DapEvent::Breakpoint { .. })
    });
    let DapEvent::Breakpoint { breakpoint, .. } = event else {
        panic!("unexpected event");
    };
    assert_eq!(breakpoint.line, Some(4));
    assert!(service.stop());
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn reports_a_missing_adapter() {
    let dir = temp_dir("missing");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    let mut configuration = config(&dir, false);
    configuration.command = "kawi-missing-debug-adapter".to_string();
    assert!(service.start_request(start_request(configuration)));
    let event = events.wait(&service, |event| matches!(event, DapEvent::Error(_)));
    let DapEvent::Error(message) = event else {
        panic!("unexpected event");
    };
    assert!(message.contains("not found"));
    wait_for_status(&mut events, &service, DebugStatus::Failed);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn step_requests_stop_again() {
    let dir = temp_dir("step");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    assert!(service.start_request(start_request(config(&dir, true))));
    wait_for_stopped(&mut events, &service);
    assert!(service.step(1, StepKind::Next));
    let event = events.wait(
        &service,
        |event| matches!(event, DapEvent::Stopped { reason, .. } if reason == "step"),
    );
    let DapEvent::Stopped { thread_id, .. } = event else {
        panic!("unexpected event");
    };
    assert_eq!(thread_id, Some(1));
    assert!(service.stop());
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn reports_threads_evaluates_expressions_and_filters() {
    let dir = temp_dir("evaluate");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    let request_log = dir.join("requests.log");
    let mut configuration = config(&dir, true);
    configuration.env.insert(
        "FAKE_DAP_REQUEST_LOG".to_string(),
        request_log.display().to_string(),
    );
    let mut request = start_request(configuration);
    request.function_breakpoints = vec![FunctionBreakpoint {
        name: "main".to_string(),
        condition: None,
        hit_condition: None,
    }];
    request.exception_filters = Some(vec!["caught".to_string()]);
    assert!(service.start_request(request));
    let filters = events.wait(&service, |event| {
        matches!(event, DapEvent::ExceptionFilters { .. })
    });
    let DapEvent::ExceptionFilters { filters } = filters else {
        panic!("unexpected event");
    };
    assert!(filters.iter().any(|filter| filter.filter == "uncaught"));
    wait_for_stopped(&mut events, &service);
    let log = fs::read_to_string(&request_log).unwrap();
    let function = log
        .find("setFunctionBreakpoints")
        .expect("function breakpoints");
    let exceptions = log
        .find("setExceptionBreakpoints")
        .expect("exception filters");
    let configuration_done = log.find("configurationDone").expect("configuration done");
    assert!(function < configuration_done);
    assert!(exceptions < configuration_done);

    assert!(service.threads());
    let threads = events.wait(&service, |event| matches!(event, DapEvent::Threads { .. }));
    let DapEvent::Threads { threads } = threads else {
        panic!("unexpected event");
    };
    assert_eq!(threads.len(), 2);
    assert_eq!(threads[1].name, "worker");

    assert!(service.evaluate("x + 1".to_string(), Some(1), "repl".to_string()));
    let evaluated = events.wait(&service, |event| {
        matches!(event, DapEvent::Evaluated { .. })
    });
    let DapEvent::Evaluated {
        expression,
        context,
        result,
        ..
    } = evaluated
    else {
        panic!("unexpected event");
    };
    assert_eq!(expression, "x + 1");
    assert_eq!(context, "repl");
    assert_eq!(result, "x + 1:repl");

    assert!(service.evaluate("missing".to_string(), Some(1), "repl".to_string()));
    let failed = events.wait(&service, |event| {
        matches!(event, DapEvent::EvaluationFailed { .. })
    });
    let DapEvent::EvaluationFailed {
        expression,
        message,
        ..
    } = failed
    else {
        panic!("unexpected event");
    };
    assert_eq!(expression, "missing");
    assert!(message.contains("unknown expression"));

    assert!(service.stop());
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn responds_to_run_in_terminal_requests() {
    let dir = temp_dir("terminal");
    let terminal_file = dir.join("terminal.json");
    let (wake, _wake_rx) = mpsc::channel();
    let service = DapService::new(wake).expect("service");
    let mut events = Events::new();
    let mut configuration = config(&dir, false);
    configuration
        .env
        .insert("FAKE_DAP_RUN_IN_TERMINAL".to_string(), "1".to_string());
    configuration.env.insert(
        "FAKE_DAP_TERMINAL_FILE".to_string(),
        terminal_file.display().to_string(),
    );
    assert!(service.start_request(start_request(configuration)));
    let request = events.wait(&service, |event| {
        matches!(event, DapEvent::RunInTerminal(_))
    });
    let DapEvent::RunInTerminal(request) = request else {
        panic!("unexpected event");
    };
    assert_eq!(request.args, ["printf", "debug-terminal"]);
    assert_eq!(
        request.env.get("KAWI_DEBUG_TEST").map(String::as_str),
        Some("1")
    );
    assert!(service.respond_run_in_terminal(request.request_seq, Some(4242), None));
    wait_for_file(&terminal_file);
    let response: serde_json::Value =
        serde_json::from_slice(&fs::read(&terminal_file).unwrap()).unwrap();
    assert_eq!(response["success"], true);
    assert_eq!(response["body"]["processId"], 4242);
    assert!(service.stop());
    wait_for_status(&mut events, &service, DebugStatus::Terminated);
    let _ = fs::remove_dir_all(dir);
}

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if path.is_file() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("file was not written: {}", path.display());
}

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
