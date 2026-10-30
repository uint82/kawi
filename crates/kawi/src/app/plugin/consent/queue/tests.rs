use super::*;
use kawi_plugin::{Capability, ChangeKind};

fn request(plugin: &str, capability: Capability) -> GrantRequest {
    request_with(plugin, &[capability])
}

fn request_with(plugin: &str, capabilities: &[Capability]) -> GrantRequest {
    GrantRequest {
        plugin: plugin.to_string(),
        changed: false,
        changed_requests: Vec::new(),
        kind: GrantKind::Call,
        request_hash: "hash".to_string(),
        items: capabilities
            .iter()
            .map(|capability| GrantItem::new(*capability, None, GrantKind::Call))
            .collect(),
    }
}

fn startup_request(plugin: &str, capability: Capability, request_hash: &str) -> GrantRequest {
    let mut request = request(plugin, capability);
    request.kind = GrantKind::Startup;
    request.request_hash = request_hash.to_string();
    request.items[0].change = Some(ChangeKind::Added);
    request
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
fn startup_requests_stay_suppressed_until_the_hash_changes() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    assert_eq!(
        queue.enqueue(startup_request("a", Capability::EditorRead, "hash-1"), now),
        Enqueue::Activated
    );
    let _ = queue.take_active();
    assert_eq!(
        queue.enqueue(startup_request("a", Capability::EditorRead, "hash-1"), now),
        Enqueue::Suppressed
    );
    assert_eq!(
        queue.enqueue(startup_request("a", Capability::EditorRead, "hash-2"), now),
        Enqueue::Activated
    );
}

#[test]
fn startup_requests_are_batched_into_one_prompt() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    let capabilities = [
        Capability::EditorRead,
        Capability::UiMessage,
        Capability::DiagnosticsPublish,
    ];
    for (index, capability) in capabilities.into_iter().enumerate() {
        let expected = if index == 0 {
            Enqueue::Activated
        } else {
            Enqueue::Merged
        };
        assert_eq!(
            queue.enqueue(startup_request("a", capability, "hash").clone(), now),
            expected
        );
    }
    assert_eq!(queue.pending_count(), 0);
    assert!(queue.has_plugin_requests("a"));
    assert_eq!(queue.active_capability(), Some(Capability::EditorRead));
    let prompt = queue.take_active().unwrap();
    assert!(prompt.is_batch());
    assert_eq!(prompt.labels().len(), capabilities.len());
}

#[test]
fn merged_startup_batches_stay_suppressed_after_a_denial() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    queue.enqueue(startup_request("a", Capability::EditorRead, "hash"), now);
    queue.enqueue(startup_request("a", Capability::EditorWrite, "hash"), now);
    let _ = queue.take_active();
    let mut batch = startup_request("a", Capability::EditorRead, "hash");
    batch.items.push(GrantItem::new(
        Capability::EditorWrite,
        Some(ChangeKind::Added),
        GrantKind::Startup,
    ));
    assert_eq!(queue.enqueue(batch, now), Enqueue::Suppressed);
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

#[test]
fn batch_items_can_be_toggled_and_set_in_bulk() {
    let now = Instant::now();
    let mut queue = GrantQueue::default();
    queue.enqueue(
        request_with("a", &[Capability::EditorRead, Capability::EditorWrite]),
        now,
    );
    let mut prompt = queue.take_active().unwrap();
    assert!(prompt.is_batch());
    prompt.toggle_selected();
    assert!(prompt.labels()[0].contains("allow once"));
    prompt.move_by(1);
    prompt.toggle_selected();
    assert!(prompt.labels()[1].contains("allow once"));
    prompt.allow_all();
    assert!(prompt
        .request()
        .items
        .iter()
        .all(|item| item.choice == GrantChoice::Always));
    prompt.deny_all();
    assert!(prompt
        .request()
        .items
        .iter()
        .all(|item| item.choice == GrantChoice::Deny));
}
