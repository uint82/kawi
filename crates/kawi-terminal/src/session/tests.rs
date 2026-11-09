use super::*;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};

fn options(command: &str) -> SpawnOptions {
    let mut options = SpawnOptions::new(std::env::temp_dir(), 10, 40);
    options.shell = "/bin/sh".to_string();
    options.command = Some(command.to_string());
    options
}

fn shell_options() -> SpawnOptions {
    let mut options = SpawnOptions::new(std::env::temp_dir(), 10, 40);
    options.shell = "/bin/sh".to_string();
    options
}

fn screen_text(session: &TerminalSession) -> String {
    let rows = session.screen().rows().saturating_sub(1);
    let cols = session.screen().cols().saturating_sub(1);
    session.screen().contents_between(0, 0, rows, cols)
}

fn wait_for_text(session: &mut TerminalSession, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        session.drain();
        if screen_text(session).contains(needle) {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("terminal never printed {needle:?}");
}

fn wait_for_completion(session: &mut TerminalSession) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && !session.is_complete() {
        session.drain();
        thread::sleep(Duration::from_millis(10));
    }
    assert!(session.is_complete());
}

fn start_with_open_descendant() -> (TerminalSession, std::sync::mpsc::Receiver<()>) {
    let (wake, wake_rx) = channel();
    let session = TerminalSession::start(&options("trap '' HUP; sleep 30 & exit 7"), wake).unwrap();
    (session, wake_rx)
}

fn wait_for_exit(session: &mut TerminalSession, wake: &std::sync::mpsc::Receiver<()>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while session.is_running() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "terminal exit was not detected");
        wake.recv_timeout(remaining)
            .expect("terminal exit did not wake the app");
        session.drain();
    }
}

#[test]
#[cfg(unix)]
fn runs_a_command_through_a_pty() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(&options("printf hello"), wake).unwrap();
    wait_for_text(&mut session, "hello");
    session.terminate();
    assert!(!session.is_running());
}

#[test]
#[cfg(unix)]
fn terminates_a_long_running_command() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(&options("sleep 30"), wake).unwrap();
    let started = Instant::now();
    session.terminate();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!session.is_running());
}

#[test]
#[cfg(unix)]
fn terminate_now_skips_the_hangup_grace() {
    let (mut session, _wake) = start_with_open_descendant();
    let started = Instant::now();
    session.terminate_now();
    assert!(started.elapsed() < Duration::from_millis(150));
    assert!(!session.is_running());
}

#[test]
#[cfg(unix)]
fn resize_updates_the_emulator_and_pty() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(&options("sleep 1"), wake).unwrap();
    session.resize(20, 60).unwrap();
    assert_eq!(session.screen().rows(), 20);
    assert_eq!(session.screen().cols(), 60);
}

#[test]
#[cfg(unix)]
fn writes_input_to_the_process() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(&options("cat"), wake).unwrap();
    session.write(b"hello\r").unwrap();
    wait_for_text(&mut session, "hello");
    session.terminate();
}

#[test]
#[cfg(unix)]
fn sets_terminal_environment() {
    let (wake, _wake_rx) = channel();
    let mut session =
        TerminalSession::start(&options("printf '%s %s' \"$TERM\" \"$COLORTERM\""), wake).unwrap();
    wait_for_text(&mut session, "xterm-256color truecolor");
    session.terminate();
}

#[test]
#[cfg(unix)]
fn resize_reaches_the_child_process() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(&shell_options(), wake).unwrap();
    session.resize(20, 60).unwrap();
    session.write(b"stty size\r").unwrap();
    wait_for_text(&mut session, "20 60");
    session.terminate();
}

#[test]
#[cfg(unix)]
fn detects_exit_without_output() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(&options("exit 3"), wake).unwrap();
    wait_for_completion(&mut session);
    assert!(!session.is_running());
    assert_eq!(session.exit_status().unwrap().exit_code(), 3);
    assert!(session.is_closed());
}

#[test]
#[cfg(unix)]
fn detects_exit_while_descendant_holds_the_pty() {
    let (mut session, wake) = start_with_open_descendant();
    wait_for_exit(&mut session, &wake);
    assert_eq!(session.exit_status().unwrap().exit_code(), 7);
    assert!(!session.is_closed());
}

#[test]
#[cfg(unix)]
fn closing_session_stops_descendants_that_hold_the_pty() {
    let (mut session, wake) = start_with_open_descendant();
    wait_for_exit(&mut session, &wake);
    let started = Instant::now();
    drop(session);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
// TODO: enable on macOS when the PTY retains output from a descendant after the session leader exits.
fn completion_waits_for_output_to_close() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(
        &options("trap '' HUP; (sleep 0.2; printf suffix) & exit 0"),
        wake,
    )
    .unwrap();
    wait_for_completion(&mut session);
    assert!(screen_text(&session).contains("suffix"));
}

#[test]
#[cfg(unix)]
fn fast_exit_drains_large_output() {
    let (wake, _wake_rx) = channel();
    let command = "i=0; while [ $i -lt 2000 ]; do printf '0123456789abcdef0123456789abcdef\\n'; i=$((i+1)); done; printf 'END-MARKER'";
    let mut session = TerminalSession::start(&options(command), wake).unwrap();
    wait_for_completion(&mut session);
    assert!(screen_text(&session).contains("END-MARKER"));
}

#[test]
#[cfg(unix)]
fn exports_editor_environment() {
    let (wake, _wake_rx) = channel();
    let mut options = options("printf '%s %s' \"$EDITOR\" \"$GIT_EDITOR\"");
    options.editor = Some("/tmp/kawi-editor".to_string());
    let mut session = TerminalSession::start(&options, wake).unwrap();
    wait_for_text(&mut session, "/tmp/kawi-editor /tmp/kawi-editor");
    session.terminate();
}

#[test]
#[cfg(unix)]
fn runs_in_the_requested_directory() {
    let (wake, _wake_rx) = channel();
    let mut options = options("pwd");
    options.cwd = std::path::PathBuf::from("/tmp");
    let mut session = TerminalSession::start(&options, wake).unwrap();
    wait_for_text(&mut session, "/tmp");
    session.terminate();
}

#[test]
#[cfg(unix)]
fn answers_terminal_queries() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(
        &options("stty raw -echo; printf '\\033[6n'; dd bs=1 count=6 2>/dev/null | od -An -tx1"),
        wake,
    )
    .unwrap();
    wait_for_text(&mut session, "1b 5b 31 3b 31 52");
    session.terminate();
}

#[test]
#[cfg(unix)]
fn exports_the_instance_socket() {
    let (wake, _wake_rx) = channel();
    let mut options = options("printf '%s' \"$KAWI_IPC_SOCKET\"");
    options.ipc_socket = Some(std::path::PathBuf::from("/tmp/kawi.sock"));
    let mut session = TerminalSession::start(&options, wake).unwrap();
    wait_for_text(&mut session, "/tmp/kawi.sock");
    session.terminate();
}

#[test]
#[cfg(unix)]
fn exposes_the_child_process_id() {
    let (wake, _wake_rx) = channel();
    let mut session = TerminalSession::start(&options("sleep 1"), wake).unwrap();
    assert!(session.process_id().is_some());
    session.terminate();
}

#[test]
#[cfg(unix)]
fn applies_extra_environment_variables() {
    let (wake, _wake_rx) = channel();
    let mut options = options("printf $KAWI_DEBUG_TEST");
    options
        .env
        .insert("KAWI_DEBUG_TEST".to_string(), "debug-value".to_string());
    let mut session = TerminalSession::start(&options, wake).unwrap();
    wait_for_text(&mut session, "debug-value");
    session.terminate();
}
