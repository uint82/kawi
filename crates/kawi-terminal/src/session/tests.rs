use super::*;
use std::sync::mpsc::channel;
use std::time::Instant;

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

fn wait_for_text(session: &mut TerminalSession, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        session.drain();
        let rows = session.screen().rows().saturating_sub(1);
        let cols = session.screen().cols().saturating_sub(1);
        if session
            .screen()
            .contents_between(0, 0, rows, cols)
            .contains(needle)
        {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("terminal never printed {needle:?}");
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
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && (session.is_running() || !session.is_closed()) {
        session.drain();
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!session.is_running());
    assert_eq!(session.exit_status().unwrap().exit_code(), 3);
    assert!(session.is_closed());
}

#[test]
#[cfg(unix)]
fn exports_editor_environment() {
    let (wake, _wake_rx) = channel();
    let mut options = options("printf '%s %s' \"$EDITOR\" \"$GIT_EDITOR\"");
    options.editor = Some(std::path::PathBuf::from("/tmp/kawi-editor"));
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
