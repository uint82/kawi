use kawi_lsp::{DiagnosticSeverity, Languages, LspEvent, LspService, ProgressEvent, ServerState};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const FAKE_SERVER: &str = env!("CARGO_BIN_EXE_fake-lsp-server");

fn temp_project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("kawi_lsp_service_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("project");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fake\"\n").expect("marker");
    root
}

fn config_file(root: &Path, mode: &str, delay: u64, pid_file: &Path) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let pid_file = escape_toml(&pid_file.to_string_lossy());
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"{mode}\"\nFAKE_LSP_INIT_DELAY_MS = \"{delay}\"\nFAKE_LSP_PID_FILE = \"{pid_file}\"\n\n[server.fake.settings]\nfake = \"yes\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn start(root: &Path, mode: &str, delay: u64, timeout: Duration) -> (LspService, PathBuf) {
    let pid_file = root.join("server.pid");
    let config = config_file(root, mode, delay, &pid_file);
    let languages = Languages::load(&config);
    let (wake, _wake_rx) = mpsc::channel();
    let service = LspService::with_initialize_timeout(languages, wake, timeout).expect("service");
    (service, pid_file)
}

fn wait_for(service: &LspService, predicate: impl Fn(&LspEvent) -> bool) -> LspEvent {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        while let Some(event) = service.try_recv() {
            if predicate(&event) {
                return event;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("event timed out");
}

fn wait_for_log(service: &LspService, expected: &str) -> LspEvent {
    wait_for(
        service,
        |event| matches!(event, LspEvent::Log { message, .. } if message == expected),
    )
}

fn wait_for_log_prefix(service: &LspService, expected: &str) -> LspEvent {
    wait_for(
        service,
        |event| matches!(event, LspEvent::Log { message, .. } if message.starts_with(expected)),
    )
}

fn wait_for_ready(service: &LspService) {
    wait_for(service, |event| {
        matches!(
            event,
            LspEvent::Status(status) if status.state == ServerState::Ready
        )
    });
}

#[test]
fn performs_full_document_sync_and_answers_server_requests() {
    let root = temp_project("full");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    let text = "fn main() {}\n";
    assert!(service.open(&file, text));
    wait_for_ready(&service);
    wait_for_log_prefix(&service, "open:1:");
    wait_for_log(&service, "response:1002:[\"yes\",null]");
    wait_for_log(&service, "requests:ok");
    assert!(service.change(&file, "fn main() { let x = 1; }\n"));
    wait_for_log(&service, "change:2:full:fn main() { let x = 1; }\n");
    assert!(service.save(&file));
    wait_for_log(&service, "save:fn main() { let x = 1; }\n");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn publishes_current_diagnostics_and_drops_stale_versions() {
    let root = temp_project("diagnostics");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    let open = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Diagnostics(diagnostics)
                if diagnostics.version == Some(1)
                    && diagnostics.diagnostics[0].message == "open-diagnostic"
        )
    });
    let LspEvent::Diagnostics(open) = open else {
        panic!("unexpected diagnostic event");
    };
    assert_eq!(open.server, "fake");
    assert_eq!(open.path, file);
    assert_eq!(open.diagnostics[0].severity, DiagnosticSeverity::Error);
    assert!(service.change(&file, "ab\n"));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut found_change = false;
    let mut found_fresh = false;
    let mut found_stale = false;
    while Instant::now() < deadline && !(found_change && found_fresh) {
        while let Some(event) = service.try_recv() {
            match event {
                LspEvent::Log { message, .. } if message == "change:2:full:ab\n" => {
                    found_change = true;
                }
                LspEvent::Diagnostics(diagnostics) => {
                    found_stale |= diagnostics
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.message == "stale");
                    found_fresh |= diagnostics.version == Some(2)
                        && diagnostics
                            .diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.message == "fresh");
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(found_change);
    assert!(found_fresh);
    assert!(!found_stale);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn reports_progress_notifications() {
    let root = temp_project("progress");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let begin = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Progress(ProgressEvent { title: Some(title), .. }) if title == "Indexing"
        )
    });
    let LspEvent::Progress(ProgressEvent {
        percentage: Some(0),
        done: false,
        ..
    }) = begin
    else {
        panic!("unexpected begin event");
    };
    let report = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Progress(ProgressEvent {
                percentage: Some(50),
                ..
            })
        )
    });
    let LspEvent::Progress(ProgressEvent { done: false, .. }) = report else {
        panic!("unexpected report event");
    };
    let end = wait_for(&service, |event| {
        matches!(event, LspEvent::Progress(ProgressEvent { done: true, .. }))
    });
    let LspEvent::Progress(ProgressEvent {
        message: Some(message),
        ..
    }) = end
    else {
        panic!("unexpected end event");
    };
    assert_eq!(message, "done");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn sends_incremental_sync_as_a_full_document_range() {
    let root = temp_project("incremental");
    let (service, _pid_file) = start(&root, "incremental", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    wait_for_log_prefix(&service, "open:1:");
    assert!(service.change(&file, "a\n𝄞"));
    wait_for_log(&service, "change:2:incremental:a\n𝄞");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
#[cfg(unix)]
fn times_out_an_unresponsive_initialization() {
    let root = temp_project("timeout");
    let (service, pid_file) = start(&root, "full", 1_000, Duration::from_millis(100));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Status(status)
                if status.state == ServerState::Failed
                    && status.message.as_deref().is_some_and(|message| message.contains("timed out"))
        )
    });
    assert!(matches!(event, LspEvent::Status(_)));
    let pid = fs::read_to_string(&pid_file).expect("pid file");
    let pid: i32 = pid.trim().parse().expect("pid");
    drop(service);
    assert_ne!(unsafe { libc::kill(pid, 0) }, 0);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
#[cfg(unix)]
fn stops_the_server_process_on_drop() {
    let root = temp_project("cleanup");
    let (service, pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let pid = fs::read_to_string(&pid_file).expect("pid file");
    let pid: i32 = pid.trim().parse().expect("pid");
    drop(service);
    assert_ne!(unsafe { libc::kill(pid, 0) }, 0);
    fs::remove_dir_all(root).expect("cleanup");
}
