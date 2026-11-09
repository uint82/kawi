#![cfg(target_os = "macos")]

use std::io::{self, ErrorKind, Read, Write};
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::process::id;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use portable_pty::{
    native_pty_system, Child, CommandBuilder, ExitStatus, MasterPty, PtyPair, PtySize, PtySystem,
};

const TERM: &str = "xterm-256color";
const STARTUP_MARKER: &[u8] = b"NORMAL";
const QUIT_INPUT: &[u8] = b":q!\r";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "kawi_pty_{}_{}",
            id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before the epoch")
                .as_nanos()
        ));
        for name in ["home", "config", "data", "state", "cache"] {
            std::fs::create_dir_all(root.join(name)).expect("failed to create the temp workspace");
        }
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn launches_and_quits_in_a_pty() {
    let workspace = Workspace::new();
    let pair = open_pty();
    let fd = pair.master.as_raw_fd().expect("pty master has no fd");
    make_nonblocking(fd);
    let mut reader = pair.master.try_clone_reader().expect("failed to read pty");
    let mut child = pair
        .slave
        .spawn_command(command(&workspace))
        .expect("failed to spawn kawi");
    drop(pair.slave);
    read_until(&mut reader, STARTUP_MARKER, STARTUP_TIMEOUT);
    write_keys(pair.master.as_ref(), QUIT_INPUT);
    let status = wait_for_exit(child.as_mut(), EXIT_TIMEOUT);
    assert!(status.success(), "kawi exited with {status:?}");
}

fn open_pty() -> PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("failed to open a pty")
}

fn command(workspace: &Workspace) -> CommandBuilder {
    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_kawi"));
    command.arg("--no-plugins");
    command.cwd(workspace.path("home"));
    command.env("TERM", TERM);
    command.env("HOME", workspace.path("home"));
    command.env("XDG_CONFIG_HOME", workspace.path("config"));
    command.env("XDG_DATA_HOME", workspace.path("data"));
    command.env("XDG_STATE_HOME", workspace.path("state"));
    command.env("XDG_CACHE_HOME", workspace.path("cache"));
    command
}

fn make_nonblocking(fd: RawFd) {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert!(
        flags >= 0,
        "failed to read pty flags: {}",
        io::Error::last_os_error()
    );
    let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    assert!(
        result >= 0,
        "failed to set pty nonblocking: {}",
        io::Error::last_os_error()
    );
}

fn read_until(reader: &mut dyn Read, marker: &[u8], timeout: Duration) {
    let deadline = Instant::now() + timeout;
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    while Instant::now() < deadline {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                output.extend_from_slice(&buffer[..count]);
                if contains(&output, marker) {
                    return;
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => thread::sleep(POLL_INTERVAL),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => panic!("failed to read kawi output: {error}"),
        }
    }
    panic!(
        "kawi did not print {:?} within {timeout:?}; output: {:?}",
        String::from_utf8_lossy(marker),
        String::from_utf8_lossy(&output)
    );
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn write_keys(master: &(dyn MasterPty + Send), input: &[u8]) {
    let mut writer = master.take_writer().expect("failed to write to pty");
    writer.write_all(input).expect("failed to send keys");
    writer.flush().expect("failed to flush keys");
}

fn wait_for_exit(child: &mut Box<dyn Child + Send + Sync>, timeout: Duration) -> ExitStatus {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().expect("failed to poll kawi") {
            return status;
        }
        thread::sleep(POLL_INTERVAL);
    }
    let _ = child.kill();
    panic!("kawi did not exit within {timeout:?}");
}
