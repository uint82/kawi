use kawi_buffer::{Buffer, BufferChangeOrigin};

#[test]
fn failed_undo_preserves_the_change_origin() {
    let mut buffer = Buffer::from_string("a");
    assert_eq!(buffer.undo(0, 0), None);
    buffer.insert_char(0, 1, 'b');
    assert_eq!(
        buffer
            .take_changes()
            .last()
            .map(|change| change.origin.clone()),
        Some(BufferChangeOrigin::User)
    );
}

#[test]
fn failed_redo_preserves_the_change_origin() {
    let mut buffer = Buffer::from_string("a");
    assert_eq!(buffer.redo(0, 0), None);
    buffer.insert_char(0, 1, 'b');
    assert_eq!(
        buffer
            .take_changes()
            .last()
            .map(|change| change.origin.clone()),
        Some(BufferChangeOrigin::User)
    );
}
