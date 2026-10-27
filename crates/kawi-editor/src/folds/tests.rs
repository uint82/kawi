use super::*;

fn manual(start: usize, end: usize, lines: usize) -> FoldState {
    let mut folds = FoldState::default();
    folds.create_manual(start, end, lines);
    folds
}

#[test]
fn manual_fold_hides_its_inner_lines() {
    let mut folds = manual(1, 4, 6);
    folds.close_all();
    assert_eq!(folds.visible_rows(0, 10, 6), vec![0, 1, 5]);
    assert_eq!(folds.visible_line_count(6), 3);
    assert_eq!(folds.display_row(5, 6), 2);
    assert_eq!(folds.buffer_row(2, 6), 5);
}

#[test]
fn opening_a_fold_restores_hidden_lines() {
    let mut folds = manual(1, 4, 6);
    folds.close_all();
    folds.open_at(1);
    assert_eq!(folds.visible_rows(0, 10, 6), vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn toggling_a_fold_moves_off_the_fold_body() {
    let mut folds = manual(1, 4, 6);
    folds.open_all();
    assert_eq!(folds.toggle_at(3), Some(1));
    assert!(folds.is_closed(1));
    assert_eq!(folds.toggle_at(1), None);
    assert!(!folds.is_closed(1));
}

#[test]
fn fold_markers_reflect_fold_state() {
    let mut folds = manual(1, 4, 6);
    assert_eq!(folds.marker(0), ' ');
    assert_eq!(folds.marker(1), '+');
    assert_eq!(folds.marker(3), ' ');
    folds.open_all();
    assert_eq!(folds.marker(1), '-');
    assert_eq!(folds.marker(3), '|');
}

#[test]
fn next_and_previous_fold_starts_skip_hidden_starts() {
    let mut folds = FoldState::default();
    folds.create_manual(1, 5, 8);
    folds.create_manual(2, 4, 8);
    folds.create_manual(6, 7, 8);
    folds.close_all();
    assert_eq!(folds.next_fold_start(0, 8), Some(1));
    assert_eq!(folds.next_fold_start(1, 8), Some(6));
    assert_eq!(folds.prev_fold_start(6), Some(1));
}

#[test]
fn max_top_counts_visible_rows() {
    let mut folds = manual(1, 4, 6);
    folds.close_all();
    assert_eq!(folds.max_top(6, 3), 0);
    assert_eq!(folds.max_top(6, 2), 1);
}

#[test]
fn deleting_lines_adjusts_manual_folds() {
    let mut folds = manual(1, 3, 6);
    folds.adjust(LineAdjust {
        line1: 3,
        line2: 3,
        amount: None,
        amount_after: -1,
    });
    assert_eq!(
        folds.folds(),
        &[Fold {
            start: 1,
            end: 2,
            depth: 1
        }]
    );
}

#[test]
fn inserting_lines_shifts_manual_folds() {
    let mut folds = manual(2, 3, 6);
    folds.adjust(LineAdjust {
        line1: 1,
        line2: usize::MAX,
        amount: Some(1),
        amount_after: 0,
    });
    assert_eq!(
        folds.folds(),
        &[Fold {
            start: 3,
            end: 4,
            depth: 1
        }]
    );
}

#[test]
fn snapshot_round_trips_manual_and_closed_folds() {
    let mut folds = manual(1, 4, 6);
    folds.close_all();
    let snapshot = folds.snapshot();
    let mut restored = FoldState::default();
    restored.restore(snapshot, 6);
    assert_eq!(restored.folds(), folds.folds());
    assert!(restored.is_closed(1));
}

#[test]
fn restore_drops_invalid_manual_folds() {
    let snapshot = FoldSnapshot {
        manual: vec![Fold::manual(3, 9), Fold::manual(2, 2)],
        closed: vec![9],
    };
    let mut folds = FoldState::default();
    folds.restore(snapshot, 5);
    assert!(folds.folds().is_empty());
    assert!(!folds.is_closed(9));
}

#[test]
fn restore_keeps_indent_closed_starts_until_refresh() {
    let mut folds = FoldState::default();
    folds.set_indent(4, 8);
    folds.set_method(FoldMethod::Indent);
    folds.restore(
        FoldSnapshot {
            manual: Vec::new(),
            closed: vec![1],
        },
        5,
    );
    assert!(folds.is_closed(1));
    folds.refresh(&Buffer::from_string("a\n    b\n    c\nd\n"));
    assert!(folds.is_closed(1));
}

#[test]
fn dropping_manual_folds_clears_manual_closed_starts() {
    let mut folds = manual(1, 3, 6);
    folds.close_all();
    folds.drop_manual();
    assert!(folds.folds().is_empty());
    assert!(!folds.is_closed(1));
}

#[test]
fn revalidate_drops_folds_past_the_line_count() {
    let mut folds = manual(1, 4, 6);
    folds.close_all();
    folds.revalidate(3);
    assert!(folds.folds().is_empty());
    assert!(!folds.is_closed(1));
}
