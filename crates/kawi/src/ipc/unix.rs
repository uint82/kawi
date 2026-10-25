use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::workspace;

const MAX_REQUEST_LINE: usize = 128;
const MAX_PATH_BYTES: usize = 64 * 1024;
const ACCEPT_POLL: Duration = Duration::from_millis(10);
const SOCKET_PATH_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaitOutcome {
    Saved,
    Aborted,
}

pub(crate) struct Connection {
    stream: UnixStream,
}

impl Connection {
    pub(crate) fn reply(self, outcome: WaitOutcome) -> io::Result<()> {
        let mut stream = self.stream;
        let response = match outcome {
            WaitOutcome::Saved => b"OK\n".as_slice(),
            WaitOutcome::Aborted => b"ABORT\n".as_slice(),
        };
        stream.write_all(response)?;
        stream.flush()?;
        stream.shutdown(std::net::Shutdown::Both)
    }
}

pub(crate) enum Event {
    OpenWait {
        path: PathBuf,
        connection: Connection,
    },
}

pub(crate) struct Server {
    path: PathBuf,
    shutdown: Arc<AtomicBool>,
    events: Receiver<Event>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub(crate) fn start(root: &Path, wake: Sender<()>) -> Result<Self> {
        let path = socket_path(root);
        let parent = path.parent().context("socket path has no parent")?;
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        set_directory_mode(parent)?;
        let _ = fs::remove_file(&path);
        let listener =
            UnixListener::bind(&path).with_context(|| format!("bind {}", path.display()))?;
        set_socket_mode(&path)?;
        listener
            .set_nonblocking(true)
            .context("set socket nonblocking")?;
        let (sender, events) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let worker = thread::Builder::new()
            .name("kawi-ipc".to_string())
            .spawn(move || accept_loop(listener, sender, wake, thread_shutdown))
            .context("spawn ipc listener")?;
        Ok(Self {
            path,
            shutdown,
            events,
            worker: Some(worker),
        })
    }

    pub(crate) fn try_recv(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn wait(root: &Path, file: &Path) -> Result<WaitOutcome> {
    let path = socket_path(root);
    let mut stream =
        UnixStream::connect(&path).with_context(|| format!("connect {}", path.display()))?;
    let bytes = file.as_os_str().as_encoded_bytes();
    writeln!(stream, "OPEN_WAIT {}", bytes.len()).context("write ipc request")?;
    stream.write_all(bytes).context("write ipc path")?;
    stream.flush().context("flush ipc request")?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .context("finish ipc request")?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .context("read ipc response")?;
    match response.trim() {
        "OK" => Ok(WaitOutcome::Saved),
        "ABORT" => Ok(WaitOutcome::Aborted),
        other => bail!("unexpected ipc response: {other}"),
    }
}

pub(crate) fn editor_command() -> Result<String> {
    wait_command("--git-editor")
}

pub(crate) fn terminal_editor_command() -> Result<String> {
    wait_command("--wait")
}

fn wait_command(flag: &str) -> Result<String> {
    let exe = std::env::current_exe().context("resolve kawi executable")?;
    Ok(format!("{} {flag}", shell_quote(&exe)))
}

fn shell_quote(path: &Path) -> String {
    let text = path.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn accept_loop(
    listener: UnixListener,
    sender: Sender<Event>,
    wake: Sender<()>,
    shutdown: Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => handle_connection(stream, &sender, &wake),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL);
            }
            Err(_) => return,
        }
    }
}

fn handle_connection(mut stream: UnixStream, sender: &Sender<Event>, wake: &Sender<()>) {
    let Ok(path) = read_request(&mut stream) else {
        let _ = stream.write_all(b"ERR\n");
        return;
    };
    let connection = Connection { stream };
    if sender.send(Event::OpenWait { path, connection }).is_ok() {
        let _ = wake.send(());
    }
}

fn read_request(stream: &mut UnixStream) -> Result<PathBuf> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .context("set ipc read timeout")?;
    let line = read_line(stream)?;
    let length = line
        .strip_prefix("OPEN_WAIT ")
        .context("invalid ipc request")?
        .parse::<usize>()
        .context("invalid ipc path length")?;
    if length > MAX_PATH_BYTES {
        bail!("ipc path is too long");
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).context("read ipc path")?;
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

fn read_line(stream: &mut UnixStream) -> Result<String> {
    let mut line = Vec::new();
    let mut byte = [0; 1];
    while line.len() < MAX_REQUEST_LINE {
        stream.read_exact(&mut byte).context("read ipc request")?;
        if byte[0] == b'\n' {
            return String::from_utf8(line).context("ipc request is not UTF-8");
        }
        line.push(byte[0]);
    }
    bail!("ipc request is too long")
}

fn socket_path(root: &Path) -> PathBuf {
    let directory = runtime_directory();
    let key = workspace::key(root);
    let candidate = directory.join("kawi").join(format!("{key}.sock"));
    if candidate.as_os_str().as_encoded_bytes().len() <= SOCKET_PATH_LIMIT {
        return candidate;
    }
    fallback_directory().join(format!("{key}.sock"))
}

fn runtime_directory() -> PathBuf {
    if let Some(directory) = std::env::var_os("XDG_RUNTIME_DIR") {
        let directory = PathBuf::from(directory);
        if directory.is_absolute() {
            return directory;
        }
    }
    fallback_directory()
}

fn fallback_directory() -> PathBuf {
    std::env::temp_dir().join(format!("kawi-{}", unsafe { libc::geteuid() }))
}

fn set_directory_mode(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("set permissions on {}", path.display()))
}

fn set_socket_mode(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("set permissions on {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::time::Instant;

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "kawi_ipc_{}_{}_{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos(),
            name
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn server_and_client_handshake() {
        let root = temp_root("handshake");
        let file = root.join("COMMIT_EDITMSG");
        fs::write(&file, "message\n").unwrap();
        let (wake, _wake_rx) = channel();
        let server = Server::start(&root, wake).unwrap();
        let client_root = root.clone();
        let client_file = file.clone();
        let client = thread::spawn(move || wait(&client_root, &client_file));
        let deadline = Instant::now() + Duration::from_secs(2);
        let event = loop {
            if let Some(event) = server.try_recv() {
                break event;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        };
        match event {
            Event::OpenWait { path, connection } => {
                assert_eq!(path, file);
                connection.reply(WaitOutcome::Saved).unwrap();
            }
        }
        assert_eq!(client.join().unwrap().unwrap(), WaitOutcome::Saved);
        let _ = fs::remove_dir_all(root);
    }
}
