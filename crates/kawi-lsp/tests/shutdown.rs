use kawi_lsp::{Languages, LspService};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

mod support;

use support::{escape_toml, temp_project, wait_for_ready, FAKE_SERVER};

const EXIT_BUDGET: Duration = Duration::from_millis(500);
const PROCESS_TIMEOUT: Duration = Duration::from_secs(2);

struct ServerOptions {
    pid_file: PathBuf,
    init_delay_ms: u64,
    ignore_shutdown: bool,
    shutdown_delay_ms: u64,
}

fn config_for(root: &Path, options: &ServerOptions) -> PathBuf {
    let path = root.join("languages.toml");
    let command = escape_toml(FAKE_SERVER);
    let pid_file = escape_toml(&options.pid_file.to_string_lossy());
    let ignore_shutdown = u8::from(options.ignore_shutdown);
    let text = format!(
        "[server.fake]\ncommand = \"{command}\"\nargs = []\n\n\
         [server.fake.env]\n\
         FAKE_LSP_SYNC = \"full\"\n\
         FAKE_LSP_INIT_DELAY_MS = \"{}\"\n\
         FAKE_LSP_IGNORE_SHUTDOWN = \"{ignore_shutdown}\"\n\
         FAKE_LSP_SHUTDOWN_DELAY_MS = \"{}\"\n\
         FAKE_LSP_PID_FILE = \"{pid_file}\"\n\n\
         [[language]]\n\
         name = \"rust\"\n\
         language_id = \"rust\"\n\
         extensions = [\"rs\"]\n\
         root_markers = [\"Cargo.toml\"]\n\
         servers = [\"fake\"]\n",
        options.init_delay_ms, options.shutdown_delay_ms
    );
    fs::write(&path, text).expect("config");
    path
}

fn start_service(config: &Path) -> LspService {
    let languages = Languages::load(config);
    let (wake, _wake_rx) = mpsc::channel();
    LspService::with_initialize_timeout(languages, wake, Duration::from_secs(5)).expect("service")
}

fn wait_for_pid(path: &Path) -> i32 {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(text) = fs::read_to_string(path) {
            if let Ok(pid) = text.trim().parse::<i32>() {
                return pid;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("server pid file was not written");
}

fn assert_process_gone(pid: i32) {
    let deadline = Instant::now() + PROCESS_TIMEOUT;
    while Instant::now() < deadline {
        if unsafe { libc::kill(pid, 0) } != 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("server process {pid} is still running");
}

#[test]
#[cfg(unix)]
fn terminate_skips_the_shutdown_handshake() {
    let root = temp_project("terminate_ignores");
    let pid_file = root.join("fake.pid");
    let config = config_for(
        &root,
        &ServerOptions {
            pid_file: pid_file.clone(),
            init_delay_ms: 0,
            ignore_shutdown: true,
            shutdown_delay_ms: 0,
        },
    );
    let service = start_service(&config);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let pid = wait_for_pid(&pid_file);
    let started = Instant::now();
    service.terminate();
    drop(service);
    assert!(
        started.elapsed() < EXIT_BUDGET,
        "termination took {:?}",
        started.elapsed()
    );
    assert_process_gone(pid);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
#[cfg(unix)]
fn drop_skips_a_slow_shutdown_response() {
    let root = temp_project("terminate_slow_shutdown");
    let pid_file = root.join("fake.pid");
    let config = config_for(
        &root,
        &ServerOptions {
            pid_file: pid_file.clone(),
            init_delay_ms: 0,
            ignore_shutdown: false,
            shutdown_delay_ms: 5_000,
        },
    );
    let service = start_service(&config);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let pid = wait_for_pid(&pid_file);
    let started = Instant::now();
    drop(service);
    assert!(
        started.elapsed() < EXIT_BUDGET,
        "drop took {:?}",
        started.elapsed()
    );
    assert_process_gone(pid);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
#[cfg(unix)]
fn terminate_stops_initializing_servers() {
    let root = temp_project("terminate_initializing");
    let pid_file = root.join("fake.pid");
    let config = config_for(
        &root,
        &ServerOptions {
            pid_file: pid_file.clone(),
            init_delay_ms: 5_000,
            ignore_shutdown: false,
            shutdown_delay_ms: 0,
        },
    );
    let service = start_service(&config);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    let pid = wait_for_pid(&pid_file);
    let started = Instant::now();
    service.terminate();
    drop(service);
    assert!(
        started.elapsed() < EXIT_BUDGET,
        "termination took {:?}",
        started.elapsed()
    );
    assert_process_gone(pid);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn terminate_without_clients_is_immediate() {
    let root = temp_project("terminate_empty");
    let pid_file = root.join("fake.pid");
    let config = config_for(
        &root,
        &ServerOptions {
            pid_file,
            init_delay_ms: 0,
            ignore_shutdown: false,
            shutdown_delay_ms: 0,
        },
    );
    let service = start_service(&config);
    let started = Instant::now();
    service.terminate();
    drop(service);
    assert!(
        started.elapsed() < EXIT_BUDGET,
        "termination took {:?}",
        started.elapsed()
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
#[cfg(unix)]
fn graceful_shutdown_still_stops_a_responsive_server() {
    let root = temp_project("shutdown_graceful");
    let pid_file = root.join("fake.pid");
    let config = config_for(
        &root,
        &ServerOptions {
            pid_file: pid_file.clone(),
            init_delay_ms: 0,
            ignore_shutdown: false,
            shutdown_delay_ms: 0,
        },
    );
    let service = start_service(&config);
    let file = root.join("src/main.rs");
    assert!(service.open(&file, "fn main() {}\n"));
    wait_for_ready(&service);
    let pid = wait_for_pid(&pid_file);
    service.shutdown();
    drop(service);
    assert_process_gone(pid);
    fs::remove_dir_all(root).expect("cleanup");
}
