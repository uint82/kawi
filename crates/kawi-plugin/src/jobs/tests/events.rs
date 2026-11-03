use super::super::events::JobEventQueue;
use crate::protocol::{
    JobExitedEvent, JobOutputEvent, JobState, JobStream, ProcessJobEvent,
    MAX_JOB_EVENT_QUEUE_BYTES, MAX_JOB_EVENT_QUEUE_ITEMS,
};

fn output(data: impl Into<String>) -> ProcessJobEvent {
    ProcessJobEvent::Output(JobOutputEvent {
        job: 1,
        stream: JobStream::Stdout,
        data: data.into(),
        truncated: false,
    })
}

fn exited() -> ProcessJobEvent {
    ProcessJobEvent::Exited(JobExitedEvent {
        job: 1,
        state: JobState::Exited,
        exit_code: Some(0),
        signal: None,
    })
}

#[test]
fn drains_events_in_order() {
    let mut queue = JobEventQueue::new();
    queue.push(output("first"));
    queue.push(output("second"));
    queue.push(exited());
    let events = queue.drain();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], output("first"));
    assert_eq!(events[1], output("second"));
    assert_eq!(events[2], exited());
    assert!(queue.drain().is_empty());
}

#[test]
fn drops_the_oldest_event_at_the_item_limit() {
    let mut queue = JobEventQueue::new();
    for index in 0..MAX_JOB_EVENT_QUEUE_ITEMS {
        queue.push(output(index.to_string()));
    }
    queue.push(exited());
    let events = queue.drain();
    assert_eq!(events.len(), MAX_JOB_EVENT_QUEUE_ITEMS);
    assert_eq!(events.first(), Some(&output("1")));
    assert_eq!(events.last(), Some(&exited()));
}

#[test]
fn drops_the_oldest_event_at_the_byte_limit() {
    let mut queue = JobEventQueue::new();
    let chunk = "x".repeat(MAX_JOB_EVENT_QUEUE_BYTES / 2 + 1);
    queue.push(output(chunk.clone()));
    queue.push(output(chunk.clone()));
    let events = queue.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], output(chunk));
}

#[test]
fn keeps_the_exit_event_when_output_overflows() {
    let mut queue = JobEventQueue::new();
    for _ in 0..MAX_JOB_EVENT_QUEUE_ITEMS {
        queue.push(output("x"));
    }
    queue.push(exited());
    let events = queue.drain();
    assert_eq!(events.last(), Some(&exited()));
}
