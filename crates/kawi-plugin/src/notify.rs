#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyOutcome {
    Queued,
    NotRunning,
    NotSubscribed,
    Denied,
    Overflow,
}
