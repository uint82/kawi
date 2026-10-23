use kawi_lsp::{
    CursorPosition, DiagnosticSeverity, FormattingSettings, Languages, LspEvent, LspService,
    ProgressEvent, ServerState,
};
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

fn config_file(root: &Path, mode: &str, delay: u64, nav_delay: u64, pid_file: &Path) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let pid_file = escape_toml(&pid_file.to_string_lossy());
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"{mode}\"\nFAKE_LSP_INIT_DELAY_MS = \"{delay}\"\nFAKE_LSP_NAV_DELAY_MS = \"{nav_delay}\"\nFAKE_LSP_PID_FILE = \"{pid_file}\"\n\n[server.fake.settings]\nfake = \"yes\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn two_server_config(root: &Path, mode: &str) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"{mode}\"\nFAKE_LSP_NAV_LINE_OFFSET = \"0\"\n\n[server.fake2]\ncommand = \"{command}\"\nargs = []\n\n[server.fake2.env]\nFAKE_LSP_SYNC = \"{mode}\"\nFAKE_LSP_NAV_LINE_OFFSET = \"10\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\", \"fake2\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn rename_config(root: &Path, rename_mode: &str, rename_delay: u64) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\nFAKE_LSP_RENAME_MODE = \"{rename_mode}\"\nFAKE_LSP_RENAME_DELAY_MS = \"{rename_delay}\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn phase_seven_config(root: &Path, pull_diagnostics: bool) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let pull = if pull_diagnostics {
        "FAKE_LSP_PULL_DIAGNOSTICS = \"1\"\n"
    } else {
        ""
    };
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\n{pull}\n[server.fake.settings]\nfake = \"yes\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn start_phase_seven(root: &Path, pull_diagnostics: bool) -> LspService {
    let config = phase_seven_config(root, pull_diagnostics);
    let languages = Languages::load(&config);
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
}

fn start_phase_seven_refresh(root: &Path) -> LspService {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\nFAKE_LSP_REFRESH_DIAGNOSTICS = \"1\"\n\n[server.fake.settings]\nfake = \"yes\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    let languages = Languages::load(&path);
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
}

fn start_rename(root: &Path, rename_mode: &str, rename_delay: u64) -> LspService {
    let config = rename_config(root, rename_mode, rename_delay);
    let languages = Languages::load(&config);
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
}

fn start_two_servers(root: &Path, mode: &str, timeout: Duration) -> LspService {
    let config = two_server_config(root, mode);
    let languages = Languages::load(&config);
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, timeout).expect("service")
}

fn start(root: &Path, mode: &str, delay: u64, timeout: Duration) -> (LspService, PathBuf) {
    start_with_nav_delay(root, mode, delay, 0, timeout)
}

fn start_with_nav_delay(
    root: &Path,
    mode: &str,
    delay: u64,
    nav_delay: u64,
    timeout: Duration,
) -> (LspService, PathBuf) {
    let pid_file = root.join("server.pid");
    let config = config_file(root, mode, delay, nav_delay, &pid_file);
    let languages = Languages::load(&config);
    let (wake, _wake_rx) = mpsc::channel();
    let service = LspService::with_initialize_timeout(languages, wake, timeout).expect("service");
    (service, pid_file)
}

fn wait_for(service: &LspService, predicate: impl Fn(&LspEvent) -> bool) -> LspEvent {
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
fn sends_incremental_sync_as_minimal_ranges() {
    let root = temp_project("incremental");
    let (service, _pid_file) = start(&root, "incremental", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    wait_for_log_prefix(&service, "open:1:");
    assert!(service.change(&file, "a\n𝄞"));
    wait_for_log(&service, "change:2:incremental:1:0:1:0:𝄞");
    assert!(service.change(&file, "aX\n𝄞"));
    wait_for_log(&service, "change:3:incremental:0:1:0:1:X");
    assert!(service.change(&file, "aX\n"));
    wait_for_log(&service, "change:4:incremental:1:0:1:2:");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn resolves_definitions_references_and_hover() {
    let root = temp_project("navigation");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let position = CursorPosition { line: 0, column: 3 };
    let request = service.definition(&file, position);
    let definition = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Definition(event) if event.request == request
        )
    });
    let LspEvent::Definition(definition) = definition else {
        panic!("unexpected definition event");
    };
    assert_eq!(definition.locations.len(), 2);
    assert_eq!(definition.locations[0].path, file);
    assert_eq!(definition.locations[0].line, 0);
    assert_eq!(definition.locations[0].column, 3);
    assert_eq!(definition.locations[1].line, 1);

    let request = service.references(&file, position);
    let references = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::References(event) if event.request == request
        )
    });
    let LspEvent::References(references) = references else {
        panic!("unexpected references event");
    };
    assert_eq!(references.locations.len(), 2);

    let request = service.hover(&file, position);
    let hover = wait_for(
        &service,
        |event| matches!(event, LspEvent::Hover(event) if event.request == request),
    );
    let LspEvent::Hover(hover) = hover else {
        panic!("unexpected hover event");
    };
    assert!(hover.contents.iter().any(|line| line == "fn fake()"));
    assert!(hover.contents.iter().any(|line| line == "Fake hover"));
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn merges_navigation_results_from_multiple_servers() {
    let root = temp_project("multi_navigation");
    let service = start_two_servers(&root, "full", Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "line\n".repeat(12)));
    wait_for_ready(&service);
    wait_for_ready(&service);
    let request = service.definition(&file, CursorPosition { line: 0, column: 0 });
    let definition = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Definition(event) if event.request == request
        )
    });
    let LspEvent::Definition(definition) = definition else {
        panic!("unexpected definition event");
    };
    assert_eq!(definition.locations.len(), 4);
    let mut lines: Vec<u32> = definition
        .locations
        .iter()
        .map(|location| location.line)
        .collect();
    lines.sort_unstable();
    assert_eq!(lines, vec![0, 1, 10, 11]);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn prepares_and_returns_multi_file_rename_edits() {
    let root = temp_project("rename");
    let file = root.join("src/main.rs");
    let other = root.join("src/other.rs");
    fs::write(&other, "fn other() {}\n").expect("other");
    let service = start_rename(&root, "prepare", 0);
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.prepare_rename(&file, CursorPosition { line: 0, column: 3 });
    let ready = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::RenameReady(ready) if ready.request == request
        )
    });
    let LspEvent::RenameReady(ready) = ready else {
        panic!("unexpected rename ready event");
    };
    assert_eq!(ready.placeholder, "fake_symbol");
    assert!(service.rename(request, "renamed"));
    wait_for_log_prefix(&service, "rename:renamed:");
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Rename(event) if event.request == request
        )
    });
    let LspEvent::Rename(event) = event else {
        panic!("unexpected rename event");
    };
    assert_eq!(event.edit.documents.len(), 2);
    let current = event
        .edit
        .documents
        .iter()
        .find(|document| document.path == file)
        .expect("current document");
    assert_eq!(current.edits.len(), 1);
    assert_eq!(current.edits[0].text, "renamed");
    let second = event
        .edit
        .documents
        .iter()
        .find(|document| document.path == other)
        .expect("other document");
    assert_eq!(second.edits[0].text, "renamed");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn falls_back_to_the_local_word_without_prepare_support() {
    let root = temp_project("rename_fallback");
    let file = root.join("src/main.rs");
    let service = start_rename(&root, "simple", 0);
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.prepare_rename(&file, CursorPosition { line: 0, column: 3 });
    let ready = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::RenameReady(ready) if ready.request == request
        )
    });
    let LspEvent::RenameReady(ready) = ready else {
        panic!("unexpected rename ready event");
    };
    assert_eq!(ready.placeholder, "main");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn cancels_prepare_rename_requests() {
    let root = temp_project("rename_cancel");
    let file = root.join("src/main.rs");
    let service = start_rename(&root, "prepare", 500);
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.prepare_rename(&file, CursorPosition { line: 0, column: 3 });
    service.cancel(request);
    wait_for_log_prefix(&service, "cancel:");
    let deadline = Instant::now() + Duration::from_millis(800);
    let mut delivered = false;
    while Instant::now() < deadline {
        while let Some(event) = service.try_recv() {
            delivered |= matches!(
                event,
                LspEvent::RenameReady(ready) if ready.request == request
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!delivered);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn rejects_workspace_edits_for_stale_open_versions() {
    let root = temp_project("rename_version");
    let file = root.join("src/main.rs");
    let service = start_rename(&root, "mismatch", 0);
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.prepare_rename(&file, CursorPosition { line: 0, column: 3 });
    wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::RenameReady(ready) if ready.request == request
        )
    });
    assert!(service.rename(request, "renamed"));
    let error = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::RequestError { request: event_request, .. } if *event_request == request
        )
    });
    let LspEvent::RequestError { message, .. } = error else {
        panic!("unexpected error event");
    };
    assert!(message.contains("version"), "{message}");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn rejects_workspace_edit_resource_operations() {
    let root = temp_project("rename_resource");
    let file = root.join("src/main.rs");
    let service = start_rename(&root, "resource", 0);
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.prepare_rename(&file, CursorPosition { line: 0, column: 3 });
    wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::RenameReady(ready) if ready.request == request
        )
    });
    assert!(service.rename(request, "renamed"));
    let error = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::RequestError { request: event_request, .. } if *event_request == request
        )
    });
    let LspEvent::RequestError { message, .. } = error else {
        panic!("unexpected error event");
    };
    assert!(message.contains("resource operations"), "{message}");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn cancels_navigation_requests() {
    let root = temp_project("cancel");
    let (service, _pid_file) = start_with_nav_delay(&root, "full", 0, 500, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.definition(&file, CursorPosition { line: 0, column: 3 });
    service.cancel(request);
    wait_for_log_prefix(&service, "cancel:");
    let deadline = Instant::now() + Duration::from_millis(800);
    let mut delivered = false;
    while Instant::now() < deadline {
        while let Some(event) = service.try_recv() {
            delivered |= matches!(
                event,
                LspEvent::Definition(event) if event.request == request
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!delivered);
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

#[test]
fn returns_completion_items_with_edits_and_snippet_expansion() {
    let root = temp_project("completion");
    let service = start_phase_seven(&root, false);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fak\n"));
    wait_for_ready(&service);
    let request = service.completion(&file, CursorPosition { line: 0, column: 3 });
    wait_for_log(&service, "completion:0:3");
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Completion(event) if event.request == request
        )
    });
    let LspEvent::Completion(event) = event else {
        panic!("unexpected completion event");
    };
    assert_eq!(event.server, "fake");
    assert_eq!(event.path, file);
    assert_eq!(event.items.len(), 3);
    let edit = event
        .items
        .iter()
        .find(|item| item.label == "fake_edit")
        .expect("edit item");
    assert_eq!(edit.primary.start, 0);
    assert_eq!(edit.primary.end, 3);
    assert_eq!(edit.primary.text, "edited");
    assert_eq!(edit.additional.len(), 1);
    assert_eq!(edit.additional[0].start, 4);
    assert_eq!(edit.additional[0].text, "// extra\n");
    let snippet = event
        .items
        .iter()
        .find(|item| item.label == "fake_snippet")
        .expect("snippet item");
    assert_eq!(snippet.primary.text, "print(value)");
    assert_eq!(snippet.primary.start, 0);
    assert_eq!(snippet.primary.end, 3);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn merges_completion_and_code_actions_from_multiple_servers() {
    let root = temp_project("multi_text");
    let service = start_two_servers(&root, "full", Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fak\n"));
    wait_for_ready(&service);
    wait_for_ready(&service);
    let request = service.completion(&file, CursorPosition { line: 0, column: 3 });
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Completion(event) if event.request == request
        )
    });
    let LspEvent::Completion(event) = event else {
        panic!("unexpected completion event");
    };
    assert_eq!(event.items.len(), 6);
    assert!(event.items.iter().any(|item| item.server == "fake"));
    assert!(event.items.iter().any(|item| item.server == "fake2"));
    let range = (
        CursorPosition { line: 0, column: 0 },
        CursorPosition { line: 0, column: 3 },
    );
    let request = service.code_actions(&file, range, Vec::new());
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::CodeActions(event) if event.request == request
        )
    });
    let LspEvent::CodeActions(event) = event else {
        panic!("unexpected code action event");
    };
    assert!(event.actions.iter().any(|action| action.server == "fake"));
    assert!(event.actions.iter().any(|action| action.server == "fake2"));
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn returns_code_actions_and_executes_commands() {
    let root = temp_project("code_actions");
    let service = start_phase_seven(&root, false);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "old\n"));
    wait_for_ready(&service);
    let range = (
        CursorPosition { line: 0, column: 0 },
        CursorPosition { line: 0, column: 3 },
    );
    let request = service.code_actions(&file, range, Vec::new());
    wait_for_log(&service, "code-action:0:0");
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::CodeActions(event) if event.request == request
        )
    });
    let LspEvent::CodeActions(event) = event else {
        panic!("unexpected code action event");
    };
    assert_eq!(event.actions.len(), 2);
    assert_eq!(event.actions[0].title, "Apply fake fix");
    assert!(event.actions[0].preferred);
    let edit = event.actions[0].edit.as_ref().expect("edit");
    assert_eq!(edit.documents[0].edits[0].text, "fixed");
    let command = event.actions[1].command.as_ref().expect("command");
    assert_eq!(command.command, "fake.command");
    assert!(service.execute_command(&file, "fake", command));
    wait_for_log_prefix(&service, "execute:fake.command:");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn returns_document_and_range_formatting_edits() {
    let root = temp_project("formatting");
    let service = start_phase_seven(&root, false);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "old\n"));
    wait_for_ready(&service);
    let settings = FormattingSettings {
        tab_size: 4,
        insert_spaces: true,
    };
    let request = service.formatting(&file, None, settings);
    wait_for_log_prefix(&service, "format:textDocument/formatting:");
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Formatting(event) if event.request == request
        )
    });
    let LspEvent::Formatting(event) = event else {
        panic!("unexpected formatting event");
    };
    assert_eq!(event.edit.documents[0].path, file);
    assert_eq!(event.edit.documents[0].edits[0].text, "formatted");
    let range = (
        CursorPosition { line: 0, column: 0 },
        CursorPosition { line: 0, column: 3 },
    );
    let request = service.formatting(&file, Some(range), settings);
    wait_for_log_prefix(&service, "format:textDocument/rangeFormatting:");
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Formatting(event) if event.request == request
        )
    });
    assert!(matches!(event, LspEvent::Formatting(_)));
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn pulls_diagnostics_and_reuses_result_ids() {
    let root = temp_project("pull_diagnostics");
    let service = start_phase_seven(&root, true);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    wait_for_log_prefix(&service, "open:1:");
    let first = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Diagnostics(diagnostics)
                if diagnostics.version == Some(1)
                    && diagnostics.diagnostics[0].message == "pull:1"
        )
    });
    let LspEvent::Diagnostics(first) = first else {
        panic!("unexpected diagnostic event");
    };
    assert_eq!(first.server, "fake");
    assert_eq!(first.diagnostics[0].source.as_deref(), Some("pull"));
    assert!(service.change(&file, "ab\n"));
    wait_for_log(&service, "diagnostic:2:pull:1");
    wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Diagnostics(diagnostics)
                if diagnostics.version == Some(2)
                    && diagnostics.diagnostics[0].message == "pull:2"
        )
    });
    assert!(service.save(&file));
    wait_for_log(&service, "diagnostic:2:pull:2");
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut delivered = false;
    while Instant::now() < deadline {
        while let Some(event) = service.try_recv() {
            delivered |= matches!(
                event,
                LspEvent::Diagnostics(diagnostics) if diagnostics.version == Some(2)
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!delivered);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn ignores_pushed_diagnostics_for_pull_servers() {
    let root = temp_project("pull_filter");
    let service = start_phase_seven(&root, true);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_pull = false;
    let mut saw_push = false;
    while Instant::now() < deadline && !saw_pull {
        while let Some(event) = service.try_recv() {
            if let LspEvent::Diagnostics(diagnostics) = event {
                let message = diagnostics
                    .diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.as_str());
                saw_pull |= message == Some("pull:1");
                saw_push |= message == Some("open-diagnostic");
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(saw_pull);
    assert!(!saw_push);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn answers_diagnostic_refresh_requests() {
    let root = temp_project("diagnostic_refresh");
    let service = start_phase_seven_refresh(&root);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    wait_for_log(&service, "requests:ok");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}
