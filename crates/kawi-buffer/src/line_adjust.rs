use ropey::Rope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineAdjust {
    pub line1: usize,
    pub line2: usize,
    pub amount: Option<isize>,
    pub amount_after: isize,
}

pub(crate) fn insertion(rope: &Rope, index: usize, text: &str) -> Option<LineAdjust> {
    let count = text.matches('\n').count();
    if count == 0 {
        return None;
    }
    let line1 = index
        .checked_sub(1)
        .map_or(1, |prev| rope.char_to_line(prev) + 2);
    Some(LineAdjust {
        line1,
        line2: usize::MAX,
        amount: Some(count as isize),
        amount_after: 0,
    })
}

pub(crate) fn char_removal(rope: &Rope, start: usize, end: usize) -> Option<LineAdjust> {
    let count = rope
        .slice(start..end)
        .chars()
        .filter(|ch| *ch == '\n')
        .count();
    if count == 0 {
        return None;
    }
    let first = rope.char_to_line(start);
    Some(LineAdjust {
        line1: first + 2,
        line2: first + 1 + count,
        amount: None,
        amount_after: -(count as isize),
    })
}

pub(crate) fn row_removal(first: usize, last: usize) -> LineAdjust {
    let count = last - first + 1;
    LineAdjust {
        line1: first + 1,
        line2: last + 1,
        amount: None,
        amount_after: -(count as isize),
    }
}

pub(crate) fn undo(old: &Rope, new: &Rope) -> Option<LineAdjust> {
    let old_len = logical_line_count(old);
    let new_len = logical_line_count(new);
    let limit = old_len.min(new_len);
    let top = common_prefix(old, new, limit);
    let suffix = common_suffix(old, new, old_len, new_len, top);
    let oldsize = old_len - top - suffix;
    let newsize = new_len - top - suffix;
    if oldsize == newsize {
        return None;
    }
    Some(LineAdjust {
        line1: top + 1,
        line2: top + oldsize,
        amount: None,
        amount_after: (newsize as isize) - (oldsize as isize),
    })
}

pub(crate) fn logical_line_count(rope: &Rope) -> usize {
    if rope.len_chars() == 0 {
        return 1;
    }
    let lines = rope.len_lines();
    if rope.get_char(rope.len_chars() - 1) == Some('\n') {
        lines.saturating_sub(1).max(1)
    } else {
        lines
    }
}

fn common_prefix(old: &Rope, new: &Rope, limit: usize) -> usize {
    let mut prefix = 0;
    while prefix < limit && line_matches(old, prefix, new, prefix) {
        prefix += 1;
    }
    prefix
}

fn common_suffix(old: &Rope, new: &Rope, old_len: usize, new_len: usize, top: usize) -> usize {
    let limit = old_len.min(new_len) - top;
    let mut suffix = 0;
    while suffix < limit && line_matches(old, old_len - 1 - suffix, new, new_len - 1 - suffix) {
        suffix += 1;
    }
    suffix
}

fn line_matches(old: &Rope, old_line: usize, new: &Rope, new_line: usize) -> bool {
    old.line(old_line).chars().eq(new.line(new_line).chars())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope_of(text: &str) -> Rope {
        Rope::from_str(text)
    }

    #[test]
    fn logical_line_count_ignores_trailing_newline() {
        assert_eq!(logical_line_count(&rope_of("")), 1);
        assert_eq!(logical_line_count(&rope_of("a")), 1);
        assert_eq!(logical_line_count(&rope_of("a\n")), 1);
        assert_eq!(logical_line_count(&rope_of("a\nb")), 2);
        assert_eq!(logical_line_count(&rope_of("a\n\n")), 2);
    }

    #[test]
    fn insertion_reports_line_after_previous_char() {
        let rope = rope_of("ab\ncd");
        let adjust = insertion(&rope, 1, "\n").unwrap();
        assert_eq!(adjust.line1, 2);
        assert_eq!(adjust.line2, usize::MAX);
        assert_eq!(adjust.amount, Some(1));
        assert_eq!(adjust.amount_after, 0);
    }

    #[test]
    fn insertion_at_buffer_start_shifts_everything() {
        let rope = rope_of("ab\ncd");
        let adjust = insertion(&rope, 0, "x\n").unwrap();
        assert_eq!(adjust.line1, 1);
        assert_eq!(adjust.amount, Some(1));
    }

    #[test]
    fn insertion_without_newline_reports_nothing() {
        let rope = rope_of("ab\ncd");
        assert_eq!(insertion(&rope, 1, "xy"), None);
    }

    #[test]
    fn char_removal_of_newline_clamps_next_line() {
        let rope = rope_of("ab\ncd");
        let adjust = char_removal(&rope, 2, 3).unwrap();
        assert_eq!(adjust.line1, 2);
        assert_eq!(adjust.line2, 2);
        assert_eq!(adjust.amount, None);
        assert_eq!(adjust.amount_after, -1);
    }

    #[test]
    fn char_removal_across_lines_clamps_lines_after_start() {
        let rope = rope_of("ab\ncd\nef");
        let adjust = char_removal(&rope, 1, 7).unwrap();
        assert_eq!(adjust.line1, 2);
        assert_eq!(adjust.line2, 3);
        assert_eq!(adjust.amount_after, -2);
    }

    #[test]
    fn char_removal_without_newline_reports_nothing() {
        let rope = rope_of("ab\ncd");
        assert_eq!(char_removal(&rope, 0, 2), None);
    }

    #[test]
    fn row_removal_reports_deleted_rows() {
        let adjust = row_removal(4, 4);
        assert_eq!(adjust.line1, 5);
        assert_eq!(adjust.line2, 5);
        assert_eq!(adjust.amount, None);
        assert_eq!(adjust.amount_after, -1);
        let adjust = row_removal(0, 2);
        assert_eq!(adjust.line1, 1);
        assert_eq!(adjust.line2, 3);
        assert_eq!(adjust.amount_after, -3);
    }

    #[test]
    fn undo_of_line_deletion_restores_following_lines() {
        let old = rope_of("a\nc");
        let new = rope_of("a\nb\nc");
        let adjust = undo(&old, &new).unwrap();
        assert_eq!(adjust.line1, 2);
        assert_eq!(adjust.line2, 1);
        assert_eq!(adjust.amount, None);
        assert_eq!(adjust.amount_after, 1);
    }

    #[test]
    fn undo_of_line_insertion_shifts_following_lines() {
        let old = rope_of("a\nb\nc");
        let new = rope_of("a\nc");
        let adjust = undo(&old, &new).unwrap();
        assert_eq!(adjust.line1, 2);
        assert_eq!(adjust.line2, 2);
        assert_eq!(adjust.amount_after, -1);
    }

    #[test]
    fn undo_with_same_line_count_reports_nothing() {
        let old = rope_of("a\nb");
        let new = rope_of("a\nx");
        assert_eq!(undo(&old, &new), None);
    }
}
