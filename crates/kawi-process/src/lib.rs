mod executable;

pub use executable::{find_executable, find_executable_in};

use std::io;
use std::process::{Child, Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSignal {
    Hangup,
    Terminate,
    Kill,
}

pub fn configure(command: &mut Command) {
    #[cfg(unix)]
    configure_unix(command);
}

pub fn configure_session(command: &mut Command) {
    #[cfg(unix)]
    configure_session_unix(command);
}

pub fn signal_group(pid: u32, signal: ProcessSignal) {
    #[cfg(unix)]
    signal_unix(pid, signal);
    #[cfg(not(unix))]
    let _ = (pid, signal);
}

pub fn terminate(child: &mut Child) {
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    signal_group(child.id(), ProcessSignal::Kill);
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn configure_unix(command: &mut Command) {
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

#[cfg(unix)]
fn configure_session_unix(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
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

#[cfg(unix)]
fn signal_unix(pid: u32, signal: ProcessSignal) {
    let Ok(pid) = i32::try_from(pid) else {
        return;
    };
    let signal = match signal {
        ProcessSignal::Hangup => libc::SIGHUP,
        ProcessSignal::Terminate => libc::SIGTERM,
        ProcessSignal::Kill => libc::SIGKILL,
    };
    unsafe {
        libc::kill(-pid, signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    #[cfg(unix)]
    fn terminate_stops_the_process_group() {
        let mut command = Command::new("cat");
        command.stdout(Stdio::null()).stderr(Stdio::null());
        configure(&mut command);
        let mut child = command.spawn().expect("spawn");
        let pid = child.id();
        terminate(&mut child);
        assert!(child.try_wait().expect("wait").is_some());
        assert_ne!(unsafe { libc::kill(pid as i32, 0) }, 0);
    }
}
