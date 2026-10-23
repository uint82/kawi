use crate::ServerConfig;
use anyhow::{Context, Result};
use std::io;
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct ServerProcess {
    child: Child,
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
        configure_command(&mut command);
        let mut child = command
            .spawn()
            .with_context(|| format!("start {}", config.command))?;
        let stdin = child.stdin.take().context("open lsp stdin")?;
        let stdout = child.stdout.take().context("open lsp stdout")?;
        let stderr = child.stderr.take().context("open lsp stderr")?;
        Ok((
            Self { child },
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
        if self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        kill_process_group(self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(unix)]
fn configure_command(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            #[cfg(target_os = "linux")]
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn configure_command(_command: &mut Command) {}

#[cfg(unix)]
fn kill_process_group(pid: u32) {
    if let Ok(pid) = i32::try_from(pid) {
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill_process_group(_pid: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

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
