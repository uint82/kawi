use crate::ServerConfig;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct ServerProcess {
    child: Child,
    detached: bool,
}

pub struct ServerIo {
    pub stdin: ChildStdin,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
}

impl ServerProcess {
    pub fn spawn(config: &ServerConfig, root: &Path) -> Result<(Self, ServerIo)> {
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .envs(&config.env)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        kawi_process::configure(&mut command);
        let mut child = command
            .spawn()
            .with_context(|| format!("start {}", config.command))?;
        let stdin = child.stdin.take().context("open lsp stdin")?;
        let stdout = child.stdout.take().context("open lsp stdout")?;
        let stderr = child.stderr.take().context("open lsp stderr")?;
        Ok((
            Self {
                child,
                detached: false,
            },
            ServerIo {
                stdin,
                stdout,
                stderr,
            },
        ))
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child.try_wait().context("poll lsp process")
    }

    pub fn wait_timeout(&mut self, timeout: Duration) -> Option<ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(Some(status)) = self.try_wait() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                self.terminate();
                return None;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn terminate(&mut self) {
        if self.detached || self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        kawi_process::terminate(&mut self.child);
    }

    pub fn detach(&mut self) {
        if self.detached {
            return;
        }
        self.detached = true;
        kawi_process::terminate_detached(&mut self.child);
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn detaches_a_spawned_process() {
        let mut config = ServerConfig::command("cat");
        config.args.clear();
        let root = std::env::temp_dir();
        let (mut process, _io) = ServerProcess::spawn(&config, &root).expect("spawn");
        let pid = process.child.id();
        process.detach();
        let deadline = Instant::now() + Duration::from_secs(1);
        while process.child.try_wait().expect("wait").is_none() {
            assert!(Instant::now() < deadline, "process was not reaped");
            thread::sleep(Duration::from_millis(5));
        }
        assert_ne!(unsafe { libc::kill(pid as i32, 0) }, 0);
    }

    #[test]
    #[cfg(unix)]
    fn terminates_a_spawned_process() {
        let mut config = ServerConfig::command("cat");
        config.args.clear();
        let root = std::env::temp_dir();
        let (mut process, _io) = ServerProcess::spawn(&config, &root).expect("spawn");
        let pid = process.child.id();
        process.terminate();
        assert!(process.try_wait().expect("wait").is_some());
        assert_ne!(unsafe { libc::kill(pid as i32, 0) }, 0);
    }
}
