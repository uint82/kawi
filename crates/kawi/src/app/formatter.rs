use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kawi_lsp::FormatterConfig;

const PATH_PLACEHOLDER: &str = "{path}";
const TEMP_PREFIX: &str = ".kawi-format-";
const TEMP_ATTEMPTS: usize = 100;
const FORMAT_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_STEP: Duration = Duration::from_millis(10);

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FormatError {
    NotFound(String),
    Failed(String),
}

pub(super) fn format(
    formatter: &FormatterConfig,
    path: &Path,
    text: &str,
) -> Result<String, FormatError> {
    let command = resolve_command(formatter)?;
    if formatter.stdin {
        format_stdin(&command, formatter, path, text)
    } else {
        format_file(&command, formatter, path, text)
    }
}

fn resolve_command(formatter: &FormatterConfig) -> Result<PathBuf, FormatError> {
    kawi_process::find_executable(&formatter.command)
        .ok_or_else(|| FormatError::NotFound(format!("formatter not found: {}", formatter.command)))
}

fn format_stdin(
    command: &Path,
    formatter: &FormatterConfig,
    path: &Path,
    text: &str,
) -> Result<String, FormatError> {
    let args = expanded_args(formatter, path);
    let output = run(command, &args, formatter, Some(text), parent_dir(path))?;
    ensure_success(&output)?;
    String::from_utf8(output.stdout)
        .map_err(|_| FormatError::Failed("formatter produced invalid UTF-8".to_string()))
}

fn format_file(
    command: &Path,
    formatter: &FormatterConfig,
    path: &Path,
    text: &str,
) -> Result<String, FormatError> {
    let temp = TempFile::create(path)?;
    fs::write(temp.path(), text)
        .map_err(|error| FormatError::Failed(format!("cannot write formatter input: {error}")))?;
    let args = expanded_args(formatter, temp.path());
    let output = run(command, &args, formatter, None, parent_dir(path))?;
    ensure_success(&output)?;
    fs::read_to_string(temp.path())
        .map_err(|error| FormatError::Failed(format!("cannot read formatter output: {error}")))
}

fn run(
    command: &Path,
    args: &[String],
    formatter: &FormatterConfig,
    input: Option<&str>,
    directory: &Path,
) -> Result<Output, FormatError> {
    let mut child = spawn(command, args, formatter, input.is_some(), directory)?;
    let stdout = take_pipe(child.stdout.take(), "stdout")?;
    let stderr = take_pipe(child.stderr.take(), "stderr")?;
    let stdout_reader = spawn_reader(stdout);
    let stderr_reader = spawn_reader(stderr);
    let writer = match input {
        Some(text) => match spawn_writer(&mut child, text) {
            Ok(writer) => Some(writer),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        },
        None => None,
    };
    let status = wait(&mut child)?;
    let stdout = join_reader(stdout_reader)?;
    let stderr = join_reader(stderr_reader)?;
    if let Some(writer) = writer {
        join_writer(writer)?;
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn spawn(
    command: &Path,
    args: &[String],
    formatter: &FormatterConfig,
    has_input: bool,
    directory: &Path,
) -> Result<Child, FormatError> {
    let stdin = if has_input {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    let mut process = Command::new(command);
    process
        .args(args)
        .envs(&formatter.env)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if directory.is_dir() {
        process.current_dir(directory);
    }
    process
        .spawn()
        .map_err(|error| FormatError::Failed(format!("cannot start formatter: {error}")))
}

fn take_pipe<T>(pipe: Option<T>, name: &str) -> Result<T, FormatError> {
    pipe.ok_or_else(|| FormatError::Failed(format!("formatter {name} is unavailable")))
}

fn spawn_reader<R>(mut reader: R) -> JoinHandle<std::io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut output = Vec::new();
        reader.read_to_end(&mut output)?;
        Ok(output)
    })
}

fn spawn_writer(
    child: &mut Child,
    text: &str,
) -> Result<JoinHandle<std::io::Result<()>>, FormatError> {
    let Some(mut stdin) = child.stdin.take() else {
        return Err(FormatError::Failed(
            "formatter stdin is unavailable".to_string(),
        ));
    };
    let text = text.to_string();
    Ok(thread::spawn(move || stdin.write_all(text.as_bytes())))
}

fn wait(child: &mut Child) -> Result<ExitStatus, FormatError> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() >= FORMAT_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FormatError::Failed("formatter timed out".to_string()));
            }
            Ok(None) => thread::sleep(WAIT_STEP),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FormatError::Failed(format!("formatter failed: {error}")));
            }
        }
    }
}

fn join_reader(handle: JoinHandle<std::io::Result<Vec<u8>>>) -> Result<Vec<u8>, FormatError> {
    handle
        .join()
        .map_err(|_| FormatError::Failed("formatter output thread panicked".to_string()))?
        .map_err(|error| FormatError::Failed(format!("cannot read formatter output: {error}")))
}

fn join_writer(handle: JoinHandle<std::io::Result<()>>) -> Result<(), FormatError> {
    match handle.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) if error.kind() == ErrorKind::BrokenPipe => Ok(()),
        Ok(Err(error)) => Err(FormatError::Failed(format!(
            "cannot write formatter input: {error}"
        ))),
        Err(_) => Err(FormatError::Failed(
            "formatter input thread panicked".to_string(),
        )),
    }
}

fn ensure_success(output: &Output) -> Result<(), FormatError> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = stderr.trim();
    if message.is_empty() {
        Err(FormatError::Failed(format!(
            "formatter exited with {}",
            output.status
        )))
    } else {
        Err(FormatError::Failed(format!("formatter failed: {message}")))
    }
}

fn expanded_args(formatter: &FormatterConfig, path: &Path) -> Vec<String> {
    let path = path.to_string_lossy();
    formatter
        .args
        .iter()
        .map(|arg| arg.replace(PATH_PLACEHOLDER, path.as_ref()))
        .collect()
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn create(source: &Path) -> Result<Self, FormatError> {
        let directory = parent_dir(source);
        let extension = source.extension().and_then(|value| value.to_str());
        for _ in 0..TEMP_ATTEMPTS {
            let path = directory.join(temp_name(extension));
            match create_file(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(FormatError::Failed(format!(
                        "cannot create formatter input: {error}"
                    )))
                }
            }
        }
        Err(FormatError::Failed(
            "cannot create formatter input".to_string(),
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn temp_name(extension: Option<&str>) -> String {
    let counter = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    match extension {
        Some(extension) if !extension.is_empty() => {
            format!("{TEMP_PREFIX}{}-{counter}.{extension}", std::process::id())
        }
        _ => format!("{TEMP_PREFIX}{}-{counter}", std::process::id()),
    }
}

fn create_file(path: &Path) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map(|_| ())
}

#[cfg(all(test, unix))]
mod tests;
