use std::time::{Duration, Instant};

use super::super::output::Utf8Stream;
use super::*;
use crate::protocol::{JobOutputEvent, ProcessJobEvent, ProcessJobParams, JOB_OUTPUT_EVENT};

#[test]
fn reports_output_before_exit() {
    let (manager, receiver) = manager("reader", &["/bin/sh"]);
    let job = open(&manager, "reader", "/bin/sh", &["-c", "printf hello"]);
    assert!(output_text(&receiver, job).contains("hello"));
    wait_exit(&receiver, job);
}

#[test]
fn output_is_capped() {
    let (manager, receiver) = manager("cap", &["/bin/sh", "yes", "head"]);
    let job = open(
        &manager,
        "cap",
        "/bin/sh",
        &["-c", "yes x | head -c 10000000"],
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut truncated = false;
    while Instant::now() < deadline {
        let event = match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => event,
            Err(_) => continue,
        };
        if let PluginHostEvent::JobEvent { event, payload, .. } = event {
            if event == JOB_OUTPUT_EVENT {
                let output: JobOutputEvent = serde_json::from_value(payload).expect("output");
                if output.job == job && output.truncated {
                    truncated = true;
                }
            }
        }
        if truncated {
            break;
        }
    }
    assert!(truncated);
}

#[test]
fn utf8_stream_preserves_split_code_points() {
    let mut stream = Utf8Stream::default();
    let bytes = "a\u{1f980}b".as_bytes();
    assert_eq!(stream.push(&bytes[..2]), "a");
    assert_eq!(stream.push(&bytes[2..3]), "");
    assert_eq!(stream.push(&bytes[3..]), "\u{1f980}b");
}

#[test]
fn utf8_stream_replaces_invalid_bytes() {
    let mut stream = Utf8Stream::default();
    assert_eq!(stream.push(&[0xff]), "\u{fffd}");
    assert!(stream.finish().is_empty());
}

#[test]
fn wasm_jobs_queue_events_until_polled() {
    let (manager, receiver) = manager("wasm-reader", &["/bin/sh"]);
    let job = open_with(
        &manager,
        &wasm_plugin("wasm-reader"),
        "/bin/sh",
        &["-c", "printf hello"],
    );
    let exited = wait_exit(&receiver, job);
    assert_eq!(exited.exit_code, Some(0));
    let events = manager
        .poll("wasm-reader", ProcessJobParams { job })
        .expect("poll");
    assert!(events
        .iter()
        .any(|event| matches!(event, ProcessJobEvent::Output(output) if output.data == "hello")));
    assert!(matches!(events.last(), Some(ProcessJobEvent::Exited(_))));
    assert!(manager
        .poll("wasm-reader", ProcessJobParams { job })
        .expect("poll")
        .is_empty());
}

#[test]
fn process_jobs_do_not_queue_events() {
    let (manager, receiver) = manager("process-reader", &["/bin/sh"]);
    let job = open(
        &manager,
        "process-reader",
        "/bin/sh",
        &["-c", "printf hello"],
    );
    wait_exit(&receiver, job);
    assert!(manager
        .poll("process-reader", ProcessJobParams { job })
        .expect("poll")
        .is_empty());
}
