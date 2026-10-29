use super::*;

fn request(plugin: &str, capability: Capability) -> GrantRequest {
    GrantRequest {
        plugin: plugin.to_string(),
        capability,
        changed: false,
        changed_requests: Vec::new(),
        kind: GrantKind::Call,
    }
}

#[test]
fn queues_one_prompt_and_deduplicates_requests() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    assert_eq!(
        queue.enqueue(request("a", Capability::EditorRead), now),
        Enqueue::Activated
    );
    assert_eq!(
        queue.enqueue(request("b", Capability::EditorRead), now),
        Enqueue::Queued
    );
    assert_eq!(
        queue.enqueue(request("a", Capability::EditorRead), now),
        Enqueue::Duplicate
    );
}

#[test]
fn startup_requests_offer_only_persistent_grants() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    let mut request = request("a", Capability::EditorRead);
    request.kind = GrantKind::Startup;
    assert_eq!(queue.enqueue(request, now), Enqueue::Activated);
    let labels = queue.items().iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(labels, ["Allow always", "Deny"]);
    let mut prompt = queue.take_active().unwrap();
    assert_eq!(prompt.choice(), GrantChoice::Deny);
    prompt.move_by(-1);
    assert_eq!(prompt.choice(), GrantChoice::Always);
}

#[test]
fn caps_the_pending_queue() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    queue.enqueue(request("active", Capability::EditorRead), now);
    for index in 0..MAX_PENDING {
        assert_eq!(
            queue.enqueue(
                request(&format!("pending-{index}"), Capability::EditorRead),
                now
            ),
            Enqueue::Queued
        );
    }
    assert_eq!(
        queue.enqueue(request("overflow", Capability::EditorRead), now),
        Enqueue::Full
    );
}

#[test]
fn expiry_lapses_the_active_prompt() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    queue.enqueue(request("a", Capability::EditorRead), now);
    assert!(queue.expire(now + PROMPT_TTL).is_some());
    assert!(queue.active().is_none());
}

#[test]
fn changed_summary_is_bounded() {
    let mut request = request("a", Capability::FsWrite);
    request.changed = true;
    request.changed_requests = (0..5).map(|index| format!("cap {index} added")).collect();
    assert_eq!(
        changed_request_summary(&request),
        " Changed: cap 0 added, cap 1 added."
    );
}
