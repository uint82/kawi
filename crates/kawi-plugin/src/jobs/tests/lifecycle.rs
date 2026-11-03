use super::*;
use crate::protocol::{
    JobState, ProcessJobParams, ProcessSignal, ProcessSignalParams, ProcessWriteParams, StdinMode,
    JOB_CLOSED, JOB_NOT_FOUND,
};

#[test]
fn streams_stdout_and_stderr_then_exits() {
    let (manager, receiver) = manager("streamer", &["/bin/sh"]);
    let job = open(
        &manager,
        "streamer",
        "/bin/sh",
        &["-c", "printf hello; printf err >&2"],
    );
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.state, JobState::Exited);
    assert_eq!(exited.exit_code, Some(0));
}

#[test]
fn writes_stdin_and_closes_it() {
    let (manager, receiver) = manager("writer", &["/bin/cat"]);
    let job = open(&manager, "writer", "/bin/cat", &[]);
    manager
        .write(
            "writer",
            ProcessWriteParams {
                job,
                data: "hello\n".to_string(),
            },
        )
        .expect("write");
    manager
        .close_stdin("writer", ProcessJobParams { job })
        .expect("close");
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.state, JobState::Exited);
}

#[test]
fn closing_stdin_twice_is_idempotent() {
    let (manager, receiver) = manager("closer", &["/bin/cat"]);
    let job = open(&manager, "closer", "/bin/cat", &[]);
    manager
        .close_stdin("closer", ProcessJobParams { job })
        .expect("close");
    manager
        .close_stdin("closer", ProcessJobParams { job })
        .expect("close");
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.exit_code, Some(0));
}

#[test]
fn closing_stdin_after_exit_is_idempotent() {
    let (manager, receiver) = manager("late-close", &["/bin/sh"]);
    let job = open(&manager, "late-close", "/bin/sh", &["-c", "true"]);
    wait_exit(&receiver, job);
    manager
        .close_stdin("late-close", ProcessJobParams { job })
        .expect("close");
}

#[test]
fn null_stdin_rejects_writes() {
    let (manager, receiver) = manager("closed", &["/bin/cat"]);
    let mut params = params("/bin/cat", &[]);
    params.stdin = StdinMode::Null;
    let job = manager.open(&plugin("closed"), params).expect("open").job;
    let error = manager
        .write(
            "closed",
            ProcessWriteParams {
                job,
                data: "hello\n".to_string(),
            },
        )
        .unwrap_err();
    assert_eq!(error.code, JOB_CLOSED);
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.exit_code, Some(0));
}

#[test]
fn kills_a_running_job() {
    let (manager, receiver) = manager("killer", &["/bin/sleep"]);
    let job = open(&manager, "killer", "/bin/sleep", &["5"]);
    manager
        .kill("killer", ProcessJobParams { job })
        .expect("kill");
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.state, JobState::Killed);
}

#[test]
fn kill_after_exit_is_idempotent() {
    let (manager, receiver) = manager("idempotent", &["/bin/sh"]);
    let job = open(&manager, "idempotent", "/bin/sh", &["-c", "true"]);
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.exit_code, Some(0));
    manager
        .kill("idempotent", ProcessJobParams { job })
        .expect("kill");
    let status = manager
        .status("idempotent", ProcessJobParams { job })
        .expect("status");
    assert_eq!(status.state, JobState::Exited);
    assert_eq!(status.exit_code, Some(0));
}

#[test]
fn signals_a_running_job() {
    let (manager, receiver) = manager("signaller", &["/bin/sleep"]);
    let job = open(&manager, "signaller", "/bin/sleep", &["5"]);
    manager
        .signal(
            "signaller",
            ProcessSignalParams {
                job,
                signal: ProcessSignal::Terminate,
            },
        )
        .expect("signal");
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.state, JobState::Exited);
    assert_eq!(exited.exit_code, None);
    assert_eq!(exited.signal, Some(15));
}

#[test]
fn signal_after_exit_is_rejected() {
    let (manager, receiver) = manager("late", &["/bin/sh"]);
    let job = open(&manager, "late", "/bin/sh", &["-c", "true"]);
    wait_exit(&receiver, job);
    let error = manager
        .signal(
            "late",
            ProcessSignalParams {
                job,
                signal: ProcessSignal::Terminate,
            },
        )
        .unwrap_err();
    assert_eq!(error.code, JOB_CLOSED);
}

#[test]
fn stop_plugin_removes_its_jobs() {
    let (manager, _receiver) = manager("stopper", &["/bin/sleep"]);
    let job = open(&manager, "stopper", "/bin/sleep", &["5"]);
    manager.stop_plugin("stopper");
    let error = manager
        .status("stopper", ProcessJobParams { job })
        .unwrap_err();
    assert_eq!(error.code, JOB_NOT_FOUND);
}

#[test]
fn status_reports_running_and_exit_code() {
    let (manager, receiver) = manager("status", &["/bin/sh"]);
    let job = open(&manager, "status", "/bin/sh", &["-c", "exit 3"]);
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.exit_code, Some(3));
    let status = manager
        .status("status", ProcessJobParams { job })
        .expect("status");
    assert_eq!(status.state, JobState::Exited);
    assert_eq!(status.exit_code, Some(3));
}

#[test]
fn job_events_are_scoped_to_the_plugin() {
    let (manager, receiver) = manager("owner", &["/bin/sh"]);
    let job = open(&manager, "owner", "/bin/sh", &["-c", "printf hello"]);
    let event = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("event");
    match event {
        PluginHostEvent::JobEvent { plugin, .. } => assert_eq!(plugin, "owner"),
        _ => panic!("unexpected event"),
    }
    manager
        .kill("other", ProcessJobParams { job })
        .expect_err("cross-plugin access");
}

#[test]
fn stop_plugin_kills_wasm_jobs() {
    let (manager, receiver) = manager("wasm-stopper", &["/bin/sleep"]);
    let job = open_with(&manager, &wasm_plugin("wasm-stopper"), "/bin/sleep", &["5"]);
    manager.stop_plugin("wasm-stopper");
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.state, JobState::Killed);
}
