use tree_sitter::{InputEdit, Point};

pub(crate) struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub(crate) fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(
            text.match_indices('\n')
                .map(|(index, _)| index.saturating_add(1)),
        );
        Self {
            starts,
            len: text.len(),
        }
    }

    pub(crate) fn line_start(&self, row: usize) -> usize {
        self.starts.get(row).copied().unwrap_or(self.len)
    }

    pub(crate) fn line_count(&self) -> usize {
        self.starts.len()
    }

    pub(crate) fn point(&self, byte: usize) -> Point {
        let byte = byte.min(self.len);
        let row = self
            .starts
            .partition_point(|start| *start <= byte)
            .saturating_sub(1);
        Point::new(row, byte.saturating_sub(self.starts[row]))
    }
}

pub(crate) fn input_edit(
    old: &str,
    new: &str,
    old_index: &LineIndex,
    new_index: &LineIndex,
) -> Option<InputEdit> {
    if old == new {
        return None;
    }
    let prefix = char_boundary(old, shared_prefix(old, new));
    let mut suffix = shared_suffix(&old[prefix..], &new[prefix..]);
    while suffix > 0
        && (!old.is_char_boundary(old.len().saturating_sub(suffix))
            || !new.is_char_boundary(new.len().saturating_sub(suffix)))
    {
        suffix -= 1;
    }
    let old_end = old.len().saturating_sub(suffix);
    let new_end = new.len().saturating_sub(suffix);
    Some(InputEdit {
        start_byte: prefix,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: old_index.point(prefix),
        old_end_position: old_index.point(old_end),
        new_end_position: new_index.point(new_end),
    })
}

fn shared_prefix(old: &str, new: &str) -> usize {
    old.bytes()
        .zip(new.bytes())
        .take_while(|(left, right)| left == right)
        .count()
}

fn shared_suffix(old: &str, new: &str) -> usize {
    old.bytes()
        .rev()
        .zip(new.bytes().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

fn char_boundary(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(old: &str, new: &str) -> Option<InputEdit> {
        input_edit(old, new, &LineIndex::new(old), &LineIndex::new(new))
    }

    #[test]
    fn insertion_has_zero_width_old_range() {
        let edit = edit("abcd", "abXcd").expect("edit");
        assert_eq!(edit.start_byte, 2);
        assert_eq!(edit.old_end_byte, 2);
        assert_eq!(edit.new_end_byte, 3);
        assert_eq!(edit.start_position, Point::new(0, 2));
        assert_eq!(edit.old_end_position, Point::new(0, 2));
        assert_eq!(edit.new_end_position, Point::new(0, 3));
    }

    #[test]
    fn deletion_has_zero_width_new_range() {
        let edit = edit("abXcd", "abcd").expect("edit");
        assert_eq!(edit.start_byte, 2);
        assert_eq!(edit.old_end_byte, 3);
        assert_eq!(edit.new_end_byte, 2);
    }

    #[test]
    fn replacement_spans_the_changed_bytes() {
        let edit = edit("abcdef", "abXYef").expect("edit");
        assert_eq!(edit.start_byte, 2);
        assert_eq!(edit.old_end_byte, 4);
        assert_eq!(edit.new_end_byte, 4);
    }

    #[test]
    fn unchanged_text_has_no_edit() {
        assert!(edit("same", "same").is_none());
    }

    #[test]
    fn unicode_edits_start_at_char_boundaries() {
        let edit = edit("aé", "aè").expect("edit");
        assert_eq!(edit.start_byte, 1);
        assert_eq!(edit.old_end_byte, 3);
        assert_eq!(edit.new_end_byte, 3);
    }

    #[test]
    fn multiline_points_track_rows_and_byte_columns() {
        let edit = edit("a\nb\nc", "a\nbx\nc").expect("edit");
        assert_eq!(edit.start_position, Point::new(1, 1));
        assert_eq!(edit.new_end_position, Point::new(1, 2));
    }

    #[test]
    fn line_index_handles_trailing_newline() {
        let index = LineIndex::new("ab\ncd\n");
        assert_eq!(index.line_count(), 3);
        assert_eq!(index.line_start(0), 0);
        assert_eq!(index.line_start(1), 3);
        assert_eq!(index.line_start(2), 6);
        assert_eq!(index.point(6), Point::new(2, 0));
    }
}
