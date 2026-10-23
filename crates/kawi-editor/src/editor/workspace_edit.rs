use kawi_buffer::TextEdit;
use ropey::Rope;

pub(super) fn remap_cursor(cursor: usize, edits: &[TextEdit], text: &Rope) -> (usize, usize) {
    let mut ordered: Vec<&TextEdit> = edits.iter().collect();
    ordered.sort_by_key(|edit| (edit.start, edit.end));
    let mut delta = 0isize;
    let mut offset = cursor;
    for edit in ordered {
        if cursor < edit.start {
            break;
        }
        let new_len = edit.text.chars().count();
        if cursor == edit.end {
            offset = edit.start.saturating_add(new_len);
            return position(text, offset.saturating_add_signed(delta));
        }
        if cursor < edit.end {
            let inside = cursor.saturating_sub(edit.start);
            offset = edit.start.saturating_add(inside);
            return position(text, offset.saturating_add_signed(delta));
        }
        delta = delta.saturating_add(new_len as isize - (edit.end - edit.start) as isize);
    }
    position(text, offset.saturating_add_signed(delta))
}

fn position(text: &Rope, offset: usize) -> (usize, usize) {
    if text.len_chars() == 0 {
        return (0, 0);
    }
    let offset = offset.min(text.len_chars());
    let row = text.char_to_line(offset);
    (row, offset.saturating_sub(text.line_to_char(row)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(start: usize, end: usize, value: &str) -> TextEdit {
        TextEdit {
            start,
            end,
            text: value.to_string(),
        }
    }

    #[test]
    fn cursor_inside_a_replacement_moves_to_the_new_text() {
        let text = Rope::from_str("one three");
        let edits = vec![edit(4, 7, "three")];
        assert_eq!(remap_cursor(5, &edits, &text), (0, 5));
        assert_eq!(remap_cursor(7, &edits, &text), (0, 9));
    }

    #[test]
    fn cursor_after_edits_shifts_by_the_delta() {
        let text = Rope::from_str("one three");
        let edits = vec![edit(4, 7, "three")];
        assert_eq!(remap_cursor(3, &edits, &text), (0, 3));
        assert_eq!(remap_cursor(0, &edits, &text), (0, 0));
    }

    #[test]
    fn cursor_tracks_prior_deletions() {
        let text = Rope::from_str("two\n3");
        let edits = vec![edit(0, 4, ""), edit(8, 13, "3")];
        assert_eq!(remap_cursor(11, &edits, &text), (1, 1));
    }

    #[test]
    fn remaps_cursors_with_unsorted_edits() {
        let text = Rope::from_str("one three");
        let edits = vec![edit(4, 9, "three"), edit(0, 3, "1")];
        assert_eq!(remap_cursor(9, &edits, &text), (0, 7));
    }

    #[test]
    fn empty_documents_remap_to_the_origin() {
        let text = Rope::from_str("");
        assert_eq!(remap_cursor(0, &[], &text), (0, 0));
    }
}
