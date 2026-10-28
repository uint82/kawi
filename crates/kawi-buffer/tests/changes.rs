use kawi_buffer::{Buffer, BufferChangeOrigin};
use ropey::Rope;

#[test]
fn records_insertions_and_removals_in_utf8_columns() {
    let mut buffer = Buffer::from_string("a中b\n");
    buffer.insert_char(0, 1, 'é');
    buffer.delete_backward(0, 2);
    let changes = buffer.take_changes();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].start.line, 0);
    assert_eq!(changes[0].start.byte_column, 1);
    assert_eq!(changes[0].text, "é");
    assert_eq!(changes[1].start.byte_column, 1);
    assert_eq!(changes[1].end.byte_column, 3);
    assert_eq!(changes[1].text, "");
    assert!(buffer.take_changes().is_empty());
}

#[test]
fn undo_changes_are_tagged_with_the_undo_origin() {
    let mut buffer = Buffer::from_string("alpha\n");
    buffer.checkpoint(0, 0);
    buffer.insert_char(0, 0, 'x');
    buffer.take_changes();
    buffer.undo(0, 1);
    let changes = buffer.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].origin, BufferChangeOrigin::Undo);
    assert_eq!(changes[0].text, "");
}

#[test]
fn replace_rope_uses_the_current_origin() {
    let mut buffer = Buffer::from_string("alpha\n");
    buffer.set_change_origin(BufferChangeOrigin::Plugin("trim".to_string()));
    buffer.replace_rope(Rope::from_str("beta\n"));
    let changes = buffer.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(
        changes[0].origin,
        BufferChangeOrigin::Plugin("trim".to_string())
    );
    assert_eq!(changes[0].text, "bet");
}
