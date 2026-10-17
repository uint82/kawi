use kawi_buffer::Buffer;

use crate::words::{self, WordClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WordSelection {
    pub anchor: (usize, usize),
    pub cursor: (usize, usize),
    pub inclusive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WordSpan {
    Charwise {
        start: (usize, usize),
        end: (usize, usize),
    },
    Linewise {
        first: usize,
        last: usize,
        col: usize,
    },
    Empty {
        at: (usize, usize),
    },
}

pub(crate) fn iw_span(buffer: &Buffer, row: usize, col: usize, count: usize) -> Option<WordSpan> {
    span(buffer, row, col, false, count)
}

pub(crate) fn aw_span(buffer: &Buffer, row: usize, col: usize, count: usize) -> Option<WordSpan> {
    span(buffer, row, col, true, count)
}

fn span(buffer: &Buffer, row: usize, col: usize, include: bool, count: usize) -> Option<WordSpan> {
    let selection = current_word(buffer, (row, col), include, count, None)?;
    Some(normal_span(buffer, selection))
}

fn normal_span(buffer: &Buffer, selection: WordSelection) -> WordSpan {
    let (start, mut end) = ordered(selection.anchor, selection.cursor);
    let mut inclusive = selection.inclusive;
    if !inclusive && end.1 == 0 && end.0 > start.0 {
        end.0 -= 1;
        if words::start_blank(buffer, start) {
            return WordSpan::Linewise {
                first: start.0,
                last: end.0,
                col: start.1,
            };
        }
        end.1 = words::line_len(buffer, end.0);
        if end.1 > 0 {
            end.1 -= 1;
            inclusive = true;
        }
    }
    if inclusive && end.1 < words::line_len(buffer, end.0) {
        end.1 += 1;
    }
    if end <= start {
        return WordSpan::Empty { at: start };
    }
    WordSpan::Charwise { start, end }
}

fn ordered(first: (usize, usize), second: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

pub(crate) fn current_word(
    buffer: &Buffer,
    cursor: (usize, usize),
    include: bool,
    count: usize,
    visual: Option<(usize, usize)>,
) -> Option<WordSelection> {
    let mut cursor = cursor;
    let mut anchor = visual.unwrap_or(cursor);
    let mut remaining = count.max(1);
    let mut inclusive = true;
    let mut include_white = false;

    if visual.is_none() || visual == Some(cursor) {
        cursor = back_in_line(buffer, cursor);
        anchor = cursor;
        if is_space(buffer, cursor) == include {
            let (pos, failed) = words::end_word_raw(buffer, cursor.0, cursor.1, true, true, 1);
            if failed {
                return None;
            }
            cursor = pos;
        } else {
            cursor = words::fwd_word_raw(buffer, cursor.0, cursor.1, true, 1).0;
            if cursor.1 == 0 {
                cursor = decl_pos(buffer, cursor).0;
            } else if let Some(pos) = oneleft(cursor) {
                cursor = pos;
            }
            if include {
                include_white = true;
            }
        }
        remaining = remaining.saturating_sub(1);
    }

    while remaining > 0 {
        inclusive = true;
        if visual.is_some() && cursor < anchor {
            let (pos, step) = decl_pos(buffer, cursor);
            if step == -1 {
                return None;
            }
            cursor = pos;
            if include != !is_space(buffer, cursor) {
                let (pos, failed) = words::bck_word_raw(buffer, cursor.0, cursor.1, true, 1);
                if failed {
                    return None;
                }
                cursor = pos;
            } else {
                let (pos, failed) = words::bckend_word_raw(buffer, cursor.0, cursor.1, true, 1);
                if failed {
                    return None;
                }
                cursor = pos;
                cursor = incl_pos(buffer, cursor).0;
            }
        } else {
            let (pos, step) = incl_pos(buffer, cursor);
            if step == -1 {
                return None;
            }
            cursor = pos;
            if include != is_space(buffer, cursor) {
                let (pos, failed) = words::fwd_word_raw(buffer, cursor.0, cursor.1, true, 1);
                cursor = pos;
                if failed && remaining > 1 {
                    return None;
                }
                if let Some(pos) = oneleft(cursor) {
                    cursor = pos;
                } else {
                    inclusive = false;
                }
            } else {
                let (pos, failed) = words::end_word_raw(buffer, cursor.0, cursor.1, true, true, 1);
                if failed {
                    return None;
                }
                cursor = pos;
            }
        }
        remaining = remaining.saturating_sub(1);
    }

    if include_white && (!is_space(buffer, cursor) || (cursor.1 == 0 && !inclusive)) {
        let saved = cursor;
        cursor = anchor;
        if let Some(back) = oneleft(cursor) {
            cursor = back_in_line(buffer, back);
            if is_space(buffer, cursor) && cursor.1 > 0 {
                anchor = cursor;
            }
        }
        cursor = saved;
    }

    Some(WordSelection {
        anchor,
        cursor,
        inclusive,
    })
}

fn incl_pos(buffer: &Buffer, pos: (usize, usize)) -> ((usize, usize), i32) {
    let (next, step) = words::inc_pos(buffer, pos.0, pos.1);
    if step >= 1 && next.1 != 0 {
        return words::inc_pos(buffer, next.0, next.1);
    }
    (next, step)
}

fn decl_pos(buffer: &Buffer, pos: (usize, usize)) -> ((usize, usize), i32) {
    let (next, step) = words::dec_pos(buffer, pos.0, pos.1);
    if step == 1 && next.1 != 0 {
        return words::dec_pos(buffer, next.0, next.1);
    }
    (next, step)
}

fn oneleft(pos: (usize, usize)) -> Option<(usize, usize)> {
    if pos.1 == 0 {
        return None;
    }
    Some((pos.0, pos.1 - 1))
}

fn back_in_line(buffer: &Buffer, pos: (usize, usize)) -> (usize, usize) {
    let class = words::class_at(buffer, pos.0, pos.1);
    let mut cursor = pos;
    while cursor.1 > 0 {
        let (next, _) = words::dec_pos(buffer, cursor.0, cursor.1);
        cursor = next;
        if words::class_at(buffer, cursor.0, cursor.1) != class {
            cursor = words::inc_pos(buffer, cursor.0, cursor.1).0;
            break;
        }
    }
    cursor
}

fn is_space(buffer: &Buffer, pos: (usize, usize)) -> bool {
    words::class_at(buffer, pos.0, pos.1) == WordClass::Space
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    #[test]
    fn iw_span_single_word() {
        assert_eq!(
            iw_span(&buf("foo bar"), 0, 1, 1),
            Some(WordSpan::Charwise {
                start: (0, 0),
                end: (0, 3)
            })
        );
    }

    #[test]
    fn iw_span_second_word() {
        assert_eq!(
            iw_span(&buf("foo bar"), 0, 5, 1),
            Some(WordSpan::Charwise {
                start: (0, 4),
                end: (0, 7)
            })
        );
    }

    #[test]
    fn aw_span_single_word_with_trailing_space() {
        assert_eq!(
            aw_span(&buf("foo bar"), 0, 1, 1),
            Some(WordSpan::Charwise {
                start: (0, 0),
                end: (0, 4)
            })
        );
    }

    #[test]
    fn iw_span_count_two_skips_space_run() {
        assert_eq!(
            iw_span(&buf("foo   \n\nbar"), 0, 3, 2),
            Some(WordSpan::Charwise {
                start: (0, 3),
                end: (1, 0)
            })
        );
    }

    #[test]
    fn iw_span_count_two_across_empty_line() {
        assert_eq!(
            iw_span(&buf("foo\n\nbar"), 1, 0, 2),
            Some(WordSpan::Charwise {
                start: (1, 0),
                end: (2, 3)
            })
        );
    }

    #[test]
    fn iw_span_count_three_across_empty_line() {
        assert_eq!(
            iw_span(&buf("foo\n\nbar baz"), 1, 0, 3),
            Some(WordSpan::Charwise {
                start: (1, 0),
                end: (2, 4)
            })
        );
    }

    #[test]
    fn aw_span_count_two_from_empty_line_is_none() {
        assert_eq!(aw_span(&buf("foo\n\nbar"), 1, 0, 2), None);
    }

    #[test]
    fn iw_span_count_two_linewise_on_indent() {
        assert_eq!(
            iw_span(&buf("  foo\n\nbar"), 0, 2, 2),
            Some(WordSpan::Linewise {
                first: 0,
                last: 1,
                col: 2
            })
        );
    }

    #[test]
    fn aw_span_count_two_on_indent_is_charwise() {
        assert_eq!(
            aw_span(&buf("  aa\n  bb\ncc"), 0, 2, 2),
            Some(WordSpan::Charwise {
                start: (0, 2),
                end: (1, 4)
            })
        );
    }

    #[test]
    fn aw_span_from_blank_line_is_charwise() {
        assert_eq!(
            aw_span(&buf("xxxx\n   \nbar"), 1, 1, 1),
            Some(WordSpan::Charwise {
                start: (1, 0),
                end: (2, 3)
            })
        );
    }

    #[test]
    fn iw_span_on_empty_line_is_empty() {
        assert_eq!(
            iw_span(&buf("foo\n\nbar"), 1, 0, 1),
            Some(WordSpan::Empty { at: (1, 0) })
        );
    }

    #[test]
    fn iw_span_on_empty_buffer_is_empty() {
        assert_eq!(
            iw_span(&buf(""), 0, 0, 1),
            Some(WordSpan::Empty { at: (0, 0) })
        );
    }
}
