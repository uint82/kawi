#![allow(dead_code)]

use kawi_dap::{
    BreakpointSet, DapEvent, DapService, DebugStatus, ExpandedConfiguration, SourceBreakpoint,
    StartRequest,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(crate) const FAKE_SERVER: &str = env!("CARGO_BIN_EXE_fake-dap-server");

pub(crate) fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kawi_dap_service_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(crate) fn config(dir: &Path, stop_on_entry: bool) -> ExpandedConfiguration {
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

pub(crate) fn start_request(config: ExpandedConfiguration) -> StartRequest {
    StartRequest {
        config,
        breakpoints: Vec::new(),
        function_breakpoints: Vec::new(),
        exception_filters: None,
    }
}

pub(crate) fn breakpoint_set(dir: &Path, lines: &[u32]) -> BreakpointSet {
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

pub(crate) struct Events {
    pub(crate) seen: Vec<DapEvent>,
}

impl Events {
    pub(crate) fn new() -> Self {
        Self { seen: Vec::new() }
    }

    pub(crate) fn wait(
        &mut self,
        service: &DapService,
        predicate: impl Fn(&DapEvent) -> bool,
    ) -> DapEvent {
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

pub(crate) fn wait_for_status(events: &mut Events, service: &DapService, status: DebugStatus) {
    events.wait(
        service,
        |event| matches!(event, DapEvent::Status(current) if *current == status),
    );
}

pub(crate) fn wait_for_stopped(events: &mut Events, service: &DapService) -> DapEvent {
    events.wait(service, |event| matches!(event, DapEvent::Stopped { .. }))
}

pub(crate) fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if path.is_file() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("file was not written: {}", path.display());
}
