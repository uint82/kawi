use kawi_buffer::Buffer;

pub fn find_motion(
    buffer: &Buffer,
    pos: (usize, usize),
    target: char,
    forward: bool,
    till: bool,
    count: usize,
    repeat: bool,
) -> Option<(usize, usize)> {
    let line: Vec<char> = buffer.line_cow(pos.0)?.chars().collect();
    if line.is_empty() {
        return None;
    }
    let mut col = pos.1.min(line.len().saturating_sub(1));
    let mut stop = !(repeat && till && count == 1);
    for _ in 0..count {
        col = find_step(&line, col, target, forward, &mut stop)?;
    }
    if till {
        col = land_before(&line, col, forward);
    }
    Some((pos.0, col))
}

pub fn find_span(
    buffer: &Buffer,
    pos: (usize, usize),
    target: char,
    forward: bool,
    till: bool,
    count: usize,
    repeat: bool,
) -> Option<((usize, usize), (usize, usize))> {
    let (end_row, end_col) = find_motion(buffer, pos, target, forward, till, count, repeat)?;
    if forward {
        Some((pos, (end_row, end_col)))
    } else if end_col >= pos.1 {
        None
    } else {
        Some(((end_row, end_col), (pos.0, pos.1.saturating_sub(1))))
    }
}

fn find_step(
    line: &[char],
    col: usize,
    target: char,
    forward: bool,
    stop: &mut bool,
) -> Option<usize> {
    let mut col = col;
    loop {
        if forward {
            col += 1;
            if col >= line.len() {
                return None;
            }
        } else if col == 0 {
            return None;
        } else {
            col -= 1;
        }
        if line[col] == target && *stop {
            return Some(col);
        }
        *stop = true;
    }
}

fn land_before(line: &[char], col: usize, forward: bool) -> usize {
    if forward {
        col.saturating_sub(1)
    } else {
        col.saturating_add(1).min(line.len().saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn lines_of(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    fn motion(
        buffer: &Buffer,
        row: usize,
        col: usize,
        target: char,
        forward: bool,
        till: bool,
        count: usize,
    ) -> Option<(usize, usize)> {
        find_motion(buffer, (row, col), target, forward, till, count, false)
    }

    #[test]
    fn forward_finds_char() {
        let lines = lines_of("abcdef");
        assert_eq!(motion(&lines, 0, 1, 'c', true, false, 1), Some((0, 2)));
    }

    #[test]
    fn forward_skips_cursor() {
        let lines = lines_of("abcdef");
        assert_eq!(motion(&lines, 0, 2, 'c', true, false, 1), None);
    }

    #[test]
    fn forward_missing_stays_none() {
        let lines = lines_of("abcdef");
        assert_eq!(motion(&lines, 0, 0, 'z', true, false, 1), None);
    }

    #[test]
    fn forward_takes_count() {
        let lines = lines_of("abcbdb");
        assert_eq!(motion(&lines, 0, 0, 'b', true, false, 2), Some((0, 3)));
    }

    #[test]
    fn backward_finds_char() {
        let lines = lines_of("abcdef");
        assert_eq!(motion(&lines, 0, 5, 'e', false, false, 1), Some((0, 4)));
    }

    #[test]
    fn backward_skips_cursor() {
        let lines = lines_of("axbxa");
        assert_eq!(motion(&lines, 0, 4, 'x', false, false, 1), Some((0, 3)));
    }

    #[test]
    fn backward_missing_stays_none() {
        let lines = lines_of("abcdef");
        assert_eq!(motion(&lines, 0, 0, 'a', false, false, 1), None);
    }

    #[test]
    fn till_forward_lands_before() {
        let lines = lines_of("abcdef");
        assert_eq!(motion(&lines, 0, 0, 'c', true, true, 1), Some((0, 1)));
    }

    #[test]
    fn till_backward_lands_after() {
        let lines = lines_of("abcdef");
        assert_eq!(motion(&lines, 0, 5, 'b', false, true, 1), Some((0, 2)));
    }

    #[test]
    fn till_adjacent_forward_stays() {
        let lines = lines_of("ax");
        assert_eq!(motion(&lines, 0, 0, 'x', true, true, 1), Some((0, 0)));
    }

    #[test]
    fn till_count_applies_once_at_end() {
        let lines = lines_of("abcbdb");
        assert_eq!(motion(&lines, 0, 0, 'b', true, true, 2), Some((0, 2)));
    }

    #[test]
    fn repeat_till_forces_first_move() {
        let lines = lines_of("axbxc");
        assert_eq!(
            find_motion(&lines, (0, 0), 'x', true, true, 1, true),
            Some((0, 2))
        );
        assert_eq!(
            find_motion(&lines, (0, 0), 'x', true, true, 1, false),
            Some((0, 0))
        );
    }

    #[test]
    fn empty_line_is_none() {
        let lines = lines_of("");
        assert_eq!(motion(&lines, 0, 0, 'x', true, false, 1), None);
    }

    #[test]
    fn span_forward_is_inclusive() {
        let lines = lines_of("axbxc");
        assert_eq!(
            find_span(&lines, (0, 0), 'x', true, false, 1, false),
            Some(((0, 0), (0, 1)))
        );
    }

    #[test]
    fn span_backward_excludes_start() {
        let lines = lines_of("axbxa");
        assert_eq!(
            find_span(&lines, (0, 4), 'x', false, false, 1, false),
            Some(((0, 3), (0, 3)))
        );
    }

    #[test]
    fn span_backward_till_adjacent_is_none() {
        let lines = lines_of("axbxa");
        assert_eq!(find_span(&lines, (0, 4), 'x', false, true, 1, false), None);
    }

    #[test]
    fn span_backward_till_with_distance() {
        let lines = lines_of("abxcx");
        assert_eq!(
            find_span(&lines, (0, 4), 'x', false, true, 1, false),
            Some(((0, 3), (0, 3)))
        );
    }
}
