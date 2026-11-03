use kawi_lsp::{
    CompletionTrigger, CursorPosition, DiagnosticMode, DiagnosticOrigin, DiagnosticSeverity,
    FileOperationKind, FormattingSettings, Languages, LspEvent, LspService, ProgressEvent,
    ServerState, SymbolKind, WorkspaceDiagnosticState,
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

fn file_operations_config(root: &Path) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\nFAKE_LSP_FILE_OPERATIONS = \"1\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn start_file_operations(root: &Path) -> LspService {
    let languages = Languages::load(&file_operations_config(root));
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
}

fn phase_seven_config(root: &Path, pull_diagnostics: bool) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let pull = if pull_diagnostics {
        "FAKE_LSP_PULL_DIAGNOSTICS = \"1\"\nFAKE_LSP_PULL_IDENTIFIER = \"fake-pull\"\n"
    } else {
        ""
    };
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\n{pull}\n[server.fake.settings]\nfake = \"yes\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn workspace_config(root: &Path) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\nFAKE_LSP_WORKSPACE_DIAGNOSTICS = \"1\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn symbol_config(root: &Path, mode: &str, resolve: bool, providers: bool, delay: u64) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\nFAKE_LSP_SYMBOL_MODE = \"{mode}\"\nFAKE_LSP_SYMBOL_RESOLVE = \"{}\"\nFAKE_LSP_SYMBOL_PROVIDERS = \"{}\"\nFAKE_LSP_SYMBOL_DELAY_MS = \"{delay}\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n",
        u8::from(resolve),
        u8::from(providers),
    );
    fs::write(&path, text).expect("config");
    path
}

fn start_symbols(
    root: &Path,
    mode: &str,
    resolve: bool,
    providers: bool,
    delay: u64,
) -> LspService {
    let config = symbol_config(root, mode, resolve, providers, delay);
    let languages = Languages::load(&config);
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
}

fn start_workspace_diagnostics(root: &Path) -> LspService {
    let languages = Languages::load(&workspace_config(root));
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
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

fn wait_for_state(service: &LspService, state: ServerState) {
    wait_for(service, |event| {
        matches!(
            event,
            LspEvent::Status(status) if status.server == "fake" && status.state == state
        )
    });
}

fn read_pid(path: &Path) -> u32 {
    fs::read_to_string(path)
        .expect("pid file")
        .trim()
        .parse()
        .expect("pid")
}

fn assert_no_start(service: &LspService, duration: Duration) {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        while let Some(event) = service.try_recv() {
            assert!(
                !matches!(
                    &event,
                    LspEvent::Status(status)
                        if status.server == "fake"
                            && matches!(
                                status.state,
                                ServerState::Starting
                                    | ServerState::Initializing
                                    | ServerState::Ready
                            )
                ),
                "unexpected start event: {event:?}"
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
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
    assert!(hover
        .contents
        .iter()
        .any(|document| document.text.contains("fn fake()")));
    assert!(hover
        .contents
        .iter()
        .any(|document| document.text.contains("Fake hover")));
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
    let request = service.completion(
        &file,
        CursorPosition { line: 0, column: 3 },
        CompletionTrigger::invoked(),
    );
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
fn resolves_completion_documentation_and_edits() {
    let root = temp_project("completion_resolve");
    let service = start_phase_seven(&root, false);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fak\n"));
    wait_for_ready(&service);
    let request = service.completion(
        &file,
        CursorPosition { line: 0, column: 3 },
        CompletionTrigger::invoked(),
    );
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Completion(event) if event.request == request
        )
    });
    let LspEvent::Completion(event) = event else {
        panic!("unexpected completion event");
    };
    let item_id = event
        .items
        .iter()
        .find_map(|item| item.resolve_id)
        .expect("resolvable item");
    let request = service.resolve_completion(item_id);
    wait_for_log(&service, "resolve");
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::CompletionResolved(event) if event.request == request
        )
    });
    let LspEvent::CompletionResolved(event) = event else {
        panic!("unexpected completion resolve event");
    };
    assert_eq!(event.item_id, item_id);
    assert_eq!(event.detail.as_deref(), Some("resolved detail"));
    let documentation = event.documentation.expect("documentation");
    assert!(documentation.markdown);
    assert!(documentation.text.contains("resolved"));
    assert_eq!(event.additional.len(), 1);
    assert_eq!(event.additional[0].text, "// resolved\n");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn returns_signature_help() {
    let root = temp_project("signature");
    let service = start_phase_seven(&root, false);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fake(\n"));
    wait_for_ready(&service);
    let request = service.signature_help(
        &file,
        CursorPosition { line: 0, column: 5 },
        CompletionTrigger::character("("),
    );
    wait_for_log(&service, "signature:0:5");
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::SignatureHelp(event) if event.request == request
        )
    });
    let LspEvent::SignatureHelp(event) = event else {
        panic!("unexpected signature help event");
    };
    assert_eq!(event.active_signature, Some(0));
    assert_eq!(event.active_parameter, Some(1));
    assert_eq!(event.signatures.len(), 1);
    assert_eq!(event.signatures[0].label, "fake(value: i32, other: i32)");
    assert_eq!(event.signatures[0].parameters[1].range, Some((17, 22)));
    assert!(event.signatures[0]
        .documentation
        .as_ref()
        .is_some_and(|documentation| documentation.markdown));
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
    let request = service.completion(
        &file,
        CursorPosition { line: 0, column: 3 },
        CompletionTrigger::invoked(),
    );
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
    assert_eq!(first.origin, DiagnosticOrigin::Pull);
    assert_eq!(first.identifier.as_deref(), Some("fake-pull"));
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
fn pulls_workspace_diagnostics_and_reuses_result_ids() {
    let root = temp_project("workspace_diagnostics");
    let service = start_workspace_diagnostics(&root);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::DiagnosticMode(event) if event.mode == DiagnosticMode::Workspace
        )
    });
    assert!(service.workspace_diagnostics());
    wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::WorkspaceDiagnostics(event)
                if event.state == WorkspaceDiagnosticState::Started
        )
    });
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::Diagnostics(diagnostics)
                if diagnostics.diagnostics.first().is_some_and(|diagnostic| {
                    diagnostic.message == "workspace:1"
                })
        )
    });
    let LspEvent::Diagnostics(event) = event else {
        panic!("unexpected workspace diagnostic event");
    };
    assert_eq!(event.path, file);
    assert_eq!(event.origin, DiagnosticOrigin::Pull);
    assert_eq!(event.identifier, None);
    assert_eq!(event.version, None);
    assert_eq!(event.diagnostics[0].source.as_deref(), Some("workspace"));
    wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::WorkspaceDiagnostics(event)
                if event.state == WorkspaceDiagnosticState::Finished
        )
    });
    assert!(service.workspace_diagnostics());
    wait_for_log(&service, "workspace-diagnostic:1");
    wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::WorkspaceDiagnostics(event)
                if event.state == WorkspaceDiagnosticState::Finished
        )
    });
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn delivers_pushed_and_pulled_diagnostics_for_pull_servers() {
    let root = temp_project("pull_filter");
    let service = start_phase_seven(&root, true);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "a\n"));
    wait_for_ready(&service);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_pull = false;
    let mut saw_push = false;
    while Instant::now() < deadline && (!saw_pull || !saw_push) {
        while let Some(event) = service.try_recv() {
            if let LspEvent::Diagnostics(diagnostics) = event {
                let message = diagnostics
                    .diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.as_str());
                if message == Some("pull:1") {
                    saw_pull |= diagnostics.origin == DiagnosticOrigin::Pull;
                } else if message == Some("open-diagnostic") {
                    saw_push |= diagnostics.origin == DiagnosticOrigin::Push
                        && diagnostics.identifier.is_none();
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(saw_pull);
    assert!(saw_push);
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

#[test]
fn requests_will_rename_and_reports_the_resulting_edit() {
    let root = temp_project("file_op_rename");
    let old = root.join("src/main.rs");
    let new = root.join("src/renamed.rs");
    fs::write(&old, "fn main() {}\n").expect("source");
    let service = start_file_operations(&root);
    assert!(service.open(&old, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.will_rename_files(&[(old.clone(), new.clone())]);
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::FileOperation(event)
                if event.request == request && event.kind == FileOperationKind::Rename
        )
    });
    let LspEvent::FileOperation(event) = event else {
        panic!("unexpected file operation event");
    };
    assert!(event.error.is_none());
    let edit = event.edit.expect("workspace edit");
    assert_eq!(edit.documents.len(), 1);
    assert_eq!(edit.documents[0].path, old);
    assert!(service.did_rename_files(&[(old, new)]));
    wait_for_log_prefix(&service, "didRename:");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn requests_will_delete_before_removing_the_file() {
    let root = temp_project("file_op_delete");
    let path = root.join("src/main.rs");
    fs::write(&path, "fn main() {}\n").expect("source");
    let service = start_file_operations(&root);
    assert!(service.open(&path, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.will_delete_files(std::slice::from_ref(&path));
    let event = wait_for(&service, |event| {
        matches!(
            event,
            LspEvent::FileOperation(event)
                if event.request == request && event.kind == FileOperationKind::Delete
        )
    });
    let LspEvent::FileOperation(event) = event else {
        panic!("unexpected file operation event");
    };
    assert!(event.error.is_none());
    assert!(event.edit.is_none());
    assert!(service.did_delete_files(std::slice::from_ref(&path)));
    wait_for_log_prefix(&service, "didDelete:");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn falls_back_to_close_and_open_without_file_operation_support() {
    let root = temp_project("file_op_fallback");
    let old = root.join("src/main.rs");
    let new = root.join("src/renamed.rs");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    assert!(service.open(&old, "fn main() {}\n"));
    wait_for_ready(&service);
    assert!(service.did_rename_files(&[(old, new)]));
    wait_for_log(&service, "close");
    wait_for_log_prefix(&service, "open:1:");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn flattens_document_symbols_with_containers_and_tags() {
    let root = temp_project("symbols_document");
    let service = start_symbols(&root, "nested", true, true, 0);
    let file = root.join("src/main.rs");
    assert!(service.open(
        &file,
        "fn outer() {\n    let inner = 1;\n}\nstruct old_struct\nfuture\n"
    ));
    wait_for_ready(&service);
    let request = service.document_symbols(&file, 7);
    let event = wait_for(
        &service,
        |event| matches!(event, LspEvent::DocumentSymbols(event) if event.request == request),
    );
    let LspEvent::DocumentSymbols(event) = event else {
        panic!("unexpected event");
    };
    assert_eq!(event.path, fs::canonicalize(&file).unwrap_or(file));
    assert_eq!(event.version, 7);
    assert_eq!(event.symbols.len(), 4);
    assert_eq!(event.symbols[0].name, "outer");
    assert_eq!(event.symbols[0].kind, SymbolKind::Function);
    assert_eq!(event.symbols[0].detail.as_deref(), Some("fn outer()"));
    assert_eq!(event.symbols[1].container.as_deref(), Some("outer"));
    assert_eq!(event.symbols[1].selection.expect("selection").column, 4);
    assert!(event.symbols[2].deprecated);
    assert_eq!(event.symbols[3].kind, SymbolKind::Unknown);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn flat_document_symbols_use_location_starts() {
    let root = temp_project("symbols_flat");
    let service = start_symbols(&root, "flat", true, true, 0);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\nlet flat_symbol = 1;\n"));
    wait_for_ready(&service);
    let request = service.document_symbols(&file, 1);
    let event = wait_for(
        &service,
        |event| matches!(event, LspEvent::DocumentSymbols(event) if event.request == request),
    );
    let LspEvent::DocumentSymbols(event) = event else {
        panic!("unexpected event");
    };
    assert_eq!(event.symbols.len(), 1);
    assert_eq!(event.symbols[0].name, "flat_symbol");
    assert_eq!(event.symbols[0].selection.expect("selection").line, 1);
    assert_eq!(event.symbols[0].container.as_deref(), Some("module"));
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn workspace_symbols_resolve_missing_ranges_and_skip_non_file_uris() {
    let root = temp_project("symbols_workspace");
    let service = start_symbols(&root, "nested", true, true, 0);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "one\ntwo\nthree\n"));
    wait_for_ready(&service);
    let request = service.workspace_symbols("work");
    let event = wait_for(
        &service,
        |event| matches!(event, LspEvent::WorkspaceSymbols(event) if event.request == request),
    );
    let LspEvent::WorkspaceSymbols(event) = event else {
        panic!("unexpected event");
    };
    assert_eq!(event.query, "work");
    assert_eq!(event.symbols.len(), 2);
    assert_eq!(event.symbols[0].name, "workspace_fn");
    assert_eq!(event.symbols[0].server.as_deref(), Some("fake"));
    let lazy = &event.symbols[1];
    assert_eq!(lazy.name, "lazy_symbol");
    assert!(!lazy.is_jumpable());
    let resolve = service.resolve_workspace_symbol(lazy.resolve_id.expect("resolve id"));
    let resolved = wait_for(
        &service,
        |event| matches!(event, LspEvent::WorkspaceSymbolResolved(event) if event.request == resolve),
    );
    let LspEvent::WorkspaceSymbolResolved(resolved) = resolved else {
        panic!("unexpected event");
    };
    assert_eq!(resolved.symbol_id, lazy.resolve_id.expect("resolve id"));
    assert!(resolved.symbol.is_jumpable());
    assert_eq!(resolved.symbol.selection.expect("selection").line, 2);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn workspace_symbols_without_resolve_keep_full_locations() {
    let root = temp_project("symbols_no_resolve");
    let service = start_symbols(&root, "nested", false, true, 0);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "one\ntwo\nthree\n"));
    wait_for_ready(&service);
    let request = service.workspace_symbols("work");
    let event = wait_for(
        &service,
        |event| matches!(event, LspEvent::WorkspaceSymbols(event) if event.request == request),
    );
    let LspEvent::WorkspaceSymbols(event) = event else {
        panic!("unexpected event");
    };
    assert_eq!(event.symbols.len(), 1);
    assert_eq!(event.symbols[0].name, "workspace_fn");
    assert!(event.symbols[0].is_jumpable());
    assert_eq!(event.symbols[0].selection.expect("selection").line, 1);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn symbol_requests_report_missing_capabilities() {
    let root = temp_project("symbols_missing");
    let service = start_symbols(&root, "nested", true, false, 0);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let document = service.document_symbols(&file, 1);
    let error = wait_for(
        &service,
        |event| matches!(event, LspEvent::RequestError { request, .. } if *request == document),
    );
    let LspEvent::RequestError { message, .. } = error else {
        panic!("unexpected event");
    };
    assert_eq!(message, "No language server supports document symbols");
    let workspace = service.workspace_symbols("main");
    let error = wait_for(
        &service,
        |event| matches!(event, LspEvent::RequestError { request, .. } if *request == workspace),
    );
    let LspEvent::RequestError { message, .. } = error else {
        panic!("unexpected event");
    };
    assert_eq!(message, "No language server supports workspace symbols");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn cancelling_workspace_symbols_sends_cancel_and_drops_the_response() {
    let root = temp_project("symbols_cancel");
    let service = start_symbols(&root, "nested", true, true, 300);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let request = service.workspace_symbols("main");
    wait_for_log(&service, "workspace-symbols:main");
    service.cancel(request);
    wait_for_log_prefix(&service, "cancel:");
    let deadline = Instant::now() + Duration::from_millis(700);
    while Instant::now() < deadline {
        if let Some(LspEvent::WorkspaceSymbols(event)) = service.try_recv() {
            panic!("received cancelled symbols: {event:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn stops_starts_and_restarts_a_client() {
    let root = temp_project("lifecycle");
    let (service, pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let first = read_pid(&pid_file);
    assert!(service.stop(Some(&file), None));
    wait_for_state(&service, ServerState::Stopped);
    assert!(service.start(Some(&file), None));
    wait_for_state(&service, ServerState::Ready);
    let second = read_pid(&pid_file);
    assert_ne!(first, second);
    assert!(service.restart(Some(&file), None));
    wait_for_state(&service, ServerState::Ready);
    let third = read_pid(&pid_file);
    assert_ne!(second, third);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn stopped_client_stays_stopped_when_reopened() {
    let root = temp_project("lifecycle_stopped");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    assert!(service.stop(Some(&file), None));
    wait_for_state(&service, ServerState::Stopped);
    assert!(service.close(&file));
    assert!(service.open(&file, "fn main() {}\n"));
    assert_no_start(&service, Duration::from_millis(300));
    assert!(service.start(Some(&file), None));
    wait_for_state(&service, ServerState::Ready);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn starts_a_named_server_without_a_buffer() {
    let root = temp_project("lifecycle_named");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    assert!(service.stop(Some(&file), None));
    wait_for_state(&service, ServerState::Stopped);
    assert!(service.start(None, Some("fake")));
    wait_for_state(&service, ServerState::Ready);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn reports_when_no_lifecycle_targets_match() {
    let root = temp_project("lifecycle_empty");
    let (service, _pid_file) = start(&root, "full", 0, Duration::from_secs(5));
    assert!(service.start(None, Some("missing")));
    let event = wait_for(
        &service,
        |event| matches!(event, LspEvent::Message { message, .. } if message.contains("no language servers")),
    );
    let LspEvent::Message { message, .. } = event else {
        panic!("unexpected event");
    };
    assert_eq!(message, "no language servers to start for missing");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}
