use super::*;

#[test]
fn session_budget_shares_remaining_across_clones() {
    let budget = SessionBudget::with_limit(10);
    let clone = budget.clone();
    budget.try_consume(6).unwrap();
    assert_eq!(clone.remaining(), 4);
    assert!(clone.try_consume(5).is_err());
    assert_eq!(budget.remaining(), 4);
}

#[test]
fn session_budget_rejects_oversized_consumption() {
    let budget = SessionBudget::with_limit(10);
    assert!(budget.try_consume(11).is_err());
    assert_eq!(budget.remaining(), 10);
}

#[test]
fn connection_budget_stops_at_limit() {
    let mut budget = ConnectionBudget::new();
    budget.take(MAX_BYTES_PER_CONNECTION as usize).unwrap();
    assert!(budget.take(1).is_err());
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn connection_budget_rejects_oversized_take() {
    let mut budget = ConnectionBudget::new();
    assert!(budget.take(MAX_BYTES_PER_CONNECTION as usize + 1).is_err());
    assert_eq!(budget.remaining(), MAX_BYTES_PER_CONNECTION);
}

#[test]
fn zero_take_does_not_consume_budget() {
    let mut connection = ConnectionBudget::new();
    connection.take(0).unwrap();
    assert_eq!(connection.remaining(), MAX_BYTES_PER_CONNECTION);

    let session = SessionBudget::with_limit(1);
    session.try_consume(0).unwrap();
    assert_eq!(session.remaining(), 1);
}

#[test]
fn network_budget_starts_with_the_session_limit() {
    let budget = NetworkBudget::new();
    assert_eq!(budget.session().remaining(), MAX_BYTES_PER_SESSION);
}
