use kawi_lsp::{LspEvent, LspService, ServerState};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const FAKE_SERVER: &str = env!("CARGO_BIN_EXE_fake-lsp-server");

pub fn temp_project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("kawi_lsp_service_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("project");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fake\"\n").expect("marker");
    root
}

pub fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn wait_for(service: &LspService, predicate: impl Fn(&LspEvent) -> bool) -> LspEvent {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        while let Some(event) = service.try_recv() {
            if predicate(&event) {
                return event;
            }
            seen.push(format!("{event:?}"));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("event timed out: {seen:?}");
}

pub fn wait_for_log_prefix(service: &LspService, expected: &str) -> LspEvent {
    wait_for(
        service,
        |event| matches!(event, LspEvent::Log { message, .. } if message.starts_with(expected)),
    )
}

pub fn wait_for_ready(service: &LspService) {
    wait_for(service, |event| {
        matches!(
            event,
            LspEvent::Status(status) if status.state == ServerState::Ready
        )
    });
}
