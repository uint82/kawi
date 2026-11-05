mod support;

use kawi_lsp::{Languages, LspEvent, LspService};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use support::{
    escape_toml, temp_project, wait_for, wait_for_log_prefix, wait_for_ready, FAKE_SERVER,
};

fn color_config(root: &Path, provider: bool, delay: u64) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let provider = u8::from(provider);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n[server.fake.env]\nFAKE_LSP_SYNC = \"full\"\nFAKE_LSP_COLOR_PROVIDER = \"{provider}\"\nFAKE_LSP_COLOR_DELAY_MS = \"{delay}\"\n\n[[language]]\nname = \"rust\"\nlanguage_id = \"rust\"\nextensions = [\"rs\"]\nroot_markers = [\"Cargo.toml\"]\nservers = [\"fake\"]\n"
    );
    fs::write(&path, text).expect("config");
    path
}

fn start_colors(root: &Path, provider: bool, delay: u64) -> LspService {
    let config = color_config(root, provider, delay);
    let languages = Languages::load(&config);
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
}

#[test]
fn requests_and_merges_document_colors() {
    let root = temp_project("colors_merge");
    let service = start_colors(&root, true, 0);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "#ff0000\nblue\n"));
    wait_for_ready(&service);
    let request = service.document_colors(&file);
    let event = wait_for(
        &service,
        |event| matches!(event, LspEvent::DocumentColors(event) if event.request == request),
    );
    let LspEvent::DocumentColors(event) = event else {
        panic!("unexpected event");
    };
    assert_eq!(event.path, file);
    assert_eq!(event.colors.len(), 2);
    assert_eq!(event.colors[0].start.column, 0);
    assert_eq!(event.colors[0].end.column, 7);
    assert_eq!(event.colors[0].red, 255);
    assert_eq!(event.colors[0].alpha, 255);
    assert_eq!(event.colors[1].start.line, 1);
    assert_eq!(event.colors[1].end.column, 4);
    assert_eq!(event.colors[1].red, 51);
    assert_eq!(event.colors[1].green, 102);
    assert_eq!(event.colors[1].blue, 153);
    assert_eq!(event.colors[1].alpha, 128);
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn document_colors_require_a_provider() {
    let root = temp_project("colors_missing");
    let service = start_colors(&root, false, 0);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "#ff0000\n"));
    wait_for_ready(&service);
    let request = service.document_colors(&file);
    let event = wait_for(
        &service,
        |event| matches!(event, LspEvent::RequestError { request: id, .. } if *id == request),
    );
    let LspEvent::RequestError { message, .. } = event else {
        panic!("unexpected event");
    };
    assert_eq!(message, "No language server supports document colors");
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn cancelling_document_colors_drops_the_response() {
    let root = temp_project("colors_cancel");
    let service = start_colors(&root, true, 300);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "#ff0000\n"));
    wait_for_ready(&service);
    let request = service.document_colors(&file);
    wait_for_log_prefix(&service, "document-colors:");
    service.cancel(request);
    wait_for_log_prefix(&service, "cancel:");
    let deadline = Instant::now() + Duration::from_millis(700);
    while Instant::now() < deadline {
        if let Some(LspEvent::DocumentColors(event)) = service.try_recv() {
            panic!("received cancelled colors: {event:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(service);
    fs::remove_dir_all(root).expect("cleanup");
}
