use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
const SOCKET_KEY_CHARS: usize = 16;
pub(crate) const SOCKET_ENV: &str = "KAWI_IPC_SOCKET";

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

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
        stream.flush()
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

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn detach(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.worker.take();
        let _ = fs::remove_file(&self.path);
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

pub(crate) fn wait(file: &Path) -> Result<WaitOutcome> {
    let path = std::env::var_os(SOCKET_ENV)
        .map(PathBuf::from)
        .context("no running kawi instance")?;
    wait_on(&path, file)
}

pub(crate) fn wait_on(path: &Path, file: &Path) -> Result<WaitOutcome> {
    let mut stream =
        UnixStream::connect(path).with_context(|| format!("connect {}", path.display()))?;
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
    let name = instance_name(root);
    let candidate = directory.join("kawi").join(&name);
    if candidate.as_os_str().as_encoded_bytes().len() <= SOCKET_PATH_LIMIT {
        return candidate;
    }
    fallback_directory().join(name)
}

fn instance_name(root: &Path) -> String {
    let key = workspace::key(root);
    let nonce = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "{}-{}-{time}-{nonce}.sock",
        &key[..SOCKET_KEY_CHARS],
        std::process::id()
    )
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
    fallback_root().join(format!("kawi-{}", unsafe { libc::geteuid() }))
}

fn fallback_root() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/tmp")
    } else {
        std::env::temp_dir()
    }
}

fn set_directory_mode(path: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if !metadata.file_type().is_dir() {
        bail!("socket directory is not a directory: {}", path.display());
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        bail!("socket directory is not owned by the current user");
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("set permissions on {}", path.display()))
}

fn set_socket_mode(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("set permissions on {}", path.display()))
}

#[cfg(test)]
mod tests;
