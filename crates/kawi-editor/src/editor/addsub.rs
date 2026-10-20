use super::Editor;
use crate::mode::Mode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumberBase {
    Decimal,
    Hex,
    Binary,
}

#[derive(Debug, Clone, Copy)]
struct NumberSpan {
    start: usize,
    value_start: usize,
    end: usize,
    base: NumberBase,
    negative: bool,
    value: u64,
    overflow: bool,
}

impl Editor {
    pub(super) fn add_to_number(&mut self, subtract: bool) {
        if self.mode.is_visual() {
            self.add_to_selection_number(subtract, false);
            return;
        }
        self.add_to_line_number(subtract);
    }

    pub(super) fn add_to_number_sequence(&mut self, subtract: bool) {
        if self.mode.is_visual() {
            self.add_to_selection_number(subtract, true);
        }
    }

    fn add_to_line_number(&mut self, subtract: bool) {
        let count = self.take_count();
        let row = self.cursor.row();
        let col = self.cursor.col();
        let chars: Vec<char> = self.buffer.line(row).unwrap_or_default().chars().collect();
        let Some(span) = find_number_at_or_after(&chars, col) else {
            return;
        };
        let text = adjust_number_text(&chars, &span, count as u64, subtract);
        self.record_boundary();
        self.put_line_text(row, span.start, span.end, &text);
        let last = span
            .start
            .saturating_add(text.chars().count().saturating_sub(1));
        self.cursor.set_position(row, last);
    }

    fn add_to_selection_number(&mut self, subtract: bool, sequence: bool) {
        self.snapshot_visual();
        let action = selection_action(subtract, sequence);
        self.note_visual_repeat(action);
        let count = self.take_count();
        let (start, end) = self.operator_range();
        let mode = self.mode;
        self.cursor.set_position(start.0, start.1);
        self.record_boundary();
        self.adjust_number_rows(start, end, mode, count, subtract, sequence);
        self.cursor.set_position(start.0, start.1);
        self.exit_visual();
    }

    pub(super) fn adjust_number_rows(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        mode: Mode,
        count: usize,
        subtract: bool,
        sequence: bool,
    ) {
        let mut amount = count as u64;
        for row in start.0..=end.0 {
            if row >= self.buffer.line_count() {
                break;
            }
            let chars: Vec<char> = self.buffer.line(row).unwrap_or_default().chars().collect();
            let Some((from, to)) = row_selection(mode, start, end, row, chars.len()) else {
                continue;
            };
            let Some(span) = find_number_in_range(&chars, from, to) else {
                continue;
            };
            let text = adjust_number_text(&chars, &span, amount, subtract);
            self.put_line_text(row, span.start, span.end, &text);
            if sequence {
                amount = amount.saturating_add(count as u64);
            }
        }
    }

    fn put_line_text(&mut self, row: usize, start: usize, end: usize, text: &str) {
        if start < end {
            self.buffer.delete_range(row, start, row, end - 1);
        }
        for (index, ch) in text.chars().enumerate() {
            self.buffer.insert_char(row, start + index, ch);
        }
    }
}

fn selection_action(subtract: bool, sequence: bool) -> crate::actions::Action {
    match (subtract, sequence) {
        (false, false) => crate::actions::Action::Increment,
        (true, false) => crate::actions::Action::Decrement,
        (false, true) => crate::actions::Action::IncrementSequence,
        (true, true) => crate::actions::Action::DecrementSequence,
    }
}

fn find_number_at_or_after(chars: &[char], cursor: usize) -> Option<NumberSpan> {
    let mut index = 0;
    while index < chars.len() {
        if let Some((end, base)) = token_at(chars, index, chars.len()) {
            if end > cursor {
                return Some(build_span(chars, index, end, base, 0));
            }
            index = end;
        } else {
            index += 1;
        }
    }
    None
}

fn find_number_in_range(chars: &[char], from: usize, to: usize) -> Option<NumberSpan> {
    let mut index = from;
    while index <= to && !chars[index].is_ascii_digit() {
        index += 1;
    }
    if index > to {
        return None;
    }
    let (end, base) = token_at(chars, index, to + 1)?;
    Some(build_span(chars, index, end, base, from))
}

fn token_at(chars: &[char], start: usize, limit: usize) -> Option<(usize, NumberBase)> {
    let first = *chars.get(start)?;
    if start + 2 < limit && first == '0' {
        let prefix = chars[start + 1];
        if matches!(prefix, 'x' | 'X') && chars[start + 2].is_ascii_hexdigit() {
            let mut end = start + 3;
            while end < limit && chars[end].is_ascii_hexdigit() {
                end += 1;
            }
            return Some((end, NumberBase::Hex));
        }
        if matches!(prefix, 'b' | 'B') && matches!(chars[start + 2], '0' | '1') {
            let mut end = start + 3;
            while end < limit && matches!(chars[end], '0' | '1') {
                end += 1;
            }
            return Some((end, NumberBase::Binary));
        }
    }
    if !first.is_ascii_digit() {
        return None;
    }
    let mut end = start + 1;
    while end < limit && chars[end].is_ascii_digit() {
        end += 1;
    }
    Some((end, NumberBase::Decimal))
}

fn build_span(
    chars: &[char],
    start: usize,
    end: usize,
    base: NumberBase,
    sign_floor: usize,
) -> NumberSpan {
    let mut negative = false;
    let mut span_start = start;
    if base == NumberBase::Decimal && start > sign_floor && chars[start - 1] == '-' {
        negative = true;
        span_start = start - 1;
    }
    let digits_start = match base {
        NumberBase::Decimal => start,
        NumberBase::Hex | NumberBase::Binary => start + 2,
    };
    let digits: String = chars[digits_start..end].iter().collect();
    let radix = match base {
        NumberBase::Decimal => 10,
        NumberBase::Hex => 16,
        NumberBase::Binary => 2,
    };
    let parsed = u64::from_str_radix(&digits, radix);
    let overflow = parsed.is_err();
    NumberSpan {
        start: span_start,
        value_start: start,
        end,
        base,
        negative,
        value: parsed.unwrap_or(u64::MAX),
        overflow,
    }
}

fn adjust_number_text(chars: &[char], span: &NumberSpan, amount: u64, subtract: bool) -> String {
    let mut negative = span.negative;
    let mut value = span.value;
    if !span.overflow {
        let old = value;
        let subtract_effective = subtract ^ negative;
        if subtract_effective {
            value = old.wrapping_sub(amount);
            if span.base == NumberBase::Decimal && value > old {
                value = 1u64.wrapping_add(!value);
                negative = !negative;
            }
        } else {
            value = old.wrapping_add(amount);
            if span.base == NumberBase::Decimal && value < old {
                value = !value;
                negative = !negative;
            }
        }
        if value == 0 {
            negative = false;
        }
    }

    let mut out = String::new();
    if negative {
        out.push('-');
    }
    let mut width = span.end - span.value_start;
    match span.base {
        NumberBase::Decimal => {}
        NumberBase::Hex | NumberBase::Binary => {
            out.push('0');
            out.push(chars[span.value_start + 1]);
            width -= 2;
        }
    }
    let digits = match span.base {
        NumberBase::Decimal => value.to_string(),
        NumberBase::Hex => {
            if hex_upper(&chars[span.start..span.end]) {
                format!("{value:X}")
            } else {
                format!("{value:x}")
            }
        }
        NumberBase::Binary => {
            if value == 0 {
                String::new()
            } else {
                format!("{value:b}")
            }
        }
    };
    if chars[span.value_start] == '0' {
        let pad = width.saturating_sub(digits.chars().count());
        for _ in 0..pad {
            out.push('0');
        }
    }
    out.push_str(&digits);
    out
}

fn hex_upper(chars: &[char]) -> bool {
    let mut upper = false;
    for ch in chars {
        if ch.is_ascii_alphabetic() {
            upper = ch.is_ascii_uppercase();
        }
    }
    upper
}

fn row_selection(
    mode: Mode,
    start: (usize, usize),
    end: (usize, usize),
    row: usize,
    line_len: usize,
) -> Option<(usize, usize)> {
    if line_len == 0 {
        return None;
    }
    let last = line_len - 1;
    match mode {
        Mode::VisualLine => Some((0, last)),
        Mode::VisualBlock => {
            let left = start.1.min(end.1);
            let right = start.1.max(end.1).min(last);
            if left > right {
                return None;
            }
            Some((left, right))
        }
        _ => {
            let from = if row == start.0 { start.1 } else { 0 };
            let to = if row == end.0 { end.1 } else { last };
            let from = from.min(last);
            let to = to.min(last);
            if from > to {
                return None;
            }
            Some((from, to))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn increment(e: &mut Editor, row: usize, col: usize, count: usize, subtract: bool) {
        e.apply(Action::JumpTo(row, col));
        if count > 1 {
            for digit in count.to_string().chars() {
                e.apply(Action::PushCount(digit.to_digit(10).unwrap() as u8));
            }
        }
        e.apply(if subtract {
            Action::Decrement
        } else {
            Action::Increment
        });
    }

    #[test]
    fn increments_number_at_or_after_cursor() {
        let mut e = editor_with("abc 5 def");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "abc 6 def");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn increments_number_under_cursor() {
        let mut e = editor_with("abc 5 def");
        increment(&mut e, 0, 4, 1, false);
        assert_eq!(e.text_lines()[0], "abc 6 def");
    }

    #[test]
    fn increments_first_number_after_cursor_on_line() {
        let mut e = editor_with("12 34");
        increment(&mut e, 0, 2, 1, false);
        assert_eq!(e.text_lines()[0], "12 35");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn no_number_is_noop() {
        let mut e = editor_with("abc def");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "abc def");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn number_ending_at_cursor_is_ignored() {
        let mut e = editor_with("5 x");
        increment(&mut e, 0, 1, 1, false);
        assert_eq!(e.text_lines()[0], "5 x");
    }

    #[test]
    fn increments_hex() {
        let mut e = editor_with("0xff");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0x100");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn increments_hex_from_prefix_and_digits() {
        for col in [0, 1, 2, 3] {
            let mut e = editor_with("0xff");
            increment(&mut e, 0, col, 1, false);
            assert_eq!(e.text_lines()[0], "0x100", "col {col}");
        }
    }

    #[test]
    fn hex_keeps_leading_zero_width() {
        let mut e = editor_with("0x0f");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0x10");
    }

    #[test]
    fn hex_pads_to_original_width() {
        let mut e = editor_with("0x00ff");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0x0100");
    }

    #[test]
    fn hex_digit_case_follows_last_alpha() {
        let mut e = editor_with("0xAF");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0xB0");
        let mut e = editor_with("0xAb");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0xac");
    }

    #[test]
    fn hex_prefix_case_is_preserved() {
        let mut e = editor_with("0Xaf");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0Xb0");
    }

    #[test]
    fn increments_binary() {
        let mut e = editor_with("0b101");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0b110");
    }

    #[test]
    fn binary_keeps_width() {
        let mut e = editor_with("0b0011");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0b0100");
    }

    #[test]
    fn binary_zero_decrement_wraps_to_all_bits() {
        let mut e = editor_with("0b0");
        increment(&mut e, 0, 0, 1, true);
        assert_eq!(e.text_lines()[0], format!("0b{}", "1".repeat(64)));
    }

    #[test]
    fn decimal_leading_zeros_are_preserved() {
        let mut e = editor_with("077");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "078");
        let mut e = editor_with("009");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "010");
        let mut e = editor_with("0000");
        increment(&mut e, 0, 0, 5, false);
        assert_eq!(e.text_lines()[0], "0005");
    }

    #[test]
    fn decimal_shortens_without_leading_zero() {
        let mut e = editor_with("100");
        increment(&mut e, 0, 0, 1, true);
        assert_eq!(e.text_lines()[0], "99");
    }

    #[test]
    fn negative_numbers_count_away_from_zero() {
        let mut e = editor_with("-5");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "-4");
        let mut e = editor_with("-5");
        increment(&mut e, 0, 1, 1, true);
        assert_eq!(e.text_lines()[0], "-6");
    }

    #[test]
    fn negative_can_become_positive() {
        let mut e = editor_with("-5");
        increment(&mut e, 0, 0, 10, false);
        assert_eq!(e.text_lines()[0], "5");
        let mut e = editor_with("-1");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "0");
    }

    #[test]
    fn zero_becomes_negative() {
        let mut e = editor_with("0");
        increment(&mut e, 0, 0, 1, true);
        assert_eq!(e.text_lines()[0], "-1");
        let mut e = editor_with("000");
        increment(&mut e, 0, 0, 1, true);
        assert_eq!(e.text_lines()[0], "-001");
        let mut e = editor_with("05");
        increment(&mut e, 0, 0, 10, true);
        assert_eq!(e.text_lines()[0], "-05");
    }

    #[test]
    fn plus_sign_is_not_part_of_number() {
        let mut e = editor_with("+5");
        increment(&mut e, 0, 0, 2, false);
        assert_eq!(e.text_lines()[0], "+7");
    }

    #[test]
    fn space_before_negative_sign_is_not_part_of_number() {
        let mut e = editor_with(" -5");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], " -4");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn hex_negative_increments_magnitude() {
        let mut e = editor_with("-0x10");
        increment(&mut e, 0, 2, 1, false);
        assert_eq!(e.text_lines()[0], "-0x11");
        let mut e = editor_with("-0xff");
        increment(&mut e, 0, 1, 1, false);
        assert_eq!(e.text_lines()[0], "-0x100");
    }

    #[test]
    fn counts_add_and_subtract() {
        let mut e = editor_with("5");
        increment(&mut e, 0, 0, 10, false);
        assert_eq!(e.text_lines()[0], "15");
        let mut e = editor_with("5");
        increment(&mut e, 0, 0, 3, true);
        assert_eq!(e.text_lines()[0], "2");
    }

    #[test]
    fn carry_extends_number() {
        let mut e = editor_with("99");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "100");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn cursor_moves_to_last_digit_of_number() {
        let mut e = editor_with("12345");
        increment(&mut e, 0, 2, 1, false);
        assert_eq!(e.text_lines()[0], "12346");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn finds_number_after_punctuation() {
        let mut e = editor_with("a.12");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "a.13");
    }

    #[test]
    fn invalid_prefix_falls_back_to_decimal() {
        let mut e = editor_with("0xGG");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "1xGG");
        let mut e = editor_with("0b2");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "1b2");
    }

    #[test]
    fn decimal_parts_are_separate_numbers() {
        let mut e = editor_with("3.14");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "4.14");
        let mut e = editor_with("3.14");
        increment(&mut e, 0, 2, 1, false);
        assert_eq!(e.text_lines()[0], "3.15");
    }

    #[test]
    fn overflow_replaces_with_maximum() {
        let mut e = editor_with("123456789012345678901234567890");
        increment(&mut e, 0, 0, 1, false);
        assert_eq!(e.text_lines()[0], "18446744073709551615");
    }

    #[test]
    fn undo_reverts_change() {
        let mut e = editor_with("5");
        increment(&mut e, 0, 0, 1, false);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "5");
    }

    #[test]
    fn repeat_applies_at_new_cursor() {
        let mut e = editor_with("5 x 7");
        increment(&mut e, 0, 0, 1, false);
        e.apply(Action::JumpTo(0, 4));
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines()[0], "6 x 8");
    }

    #[test]
    fn repeat_keeps_count() {
        let mut e = editor_with("5 x 7");
        increment(&mut e, 0, 0, 3, false);
        e.apply(Action::JumpTo(0, 4));
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines()[0], "8 x 10");
    }

    fn select_lines(e: &mut Editor, start_row: usize, end_row: usize) {
        e.apply(Action::JumpTo(start_row, 0));
        e.apply(Action::EnterVisualLine);
        for _ in start_row..end_row {
            e.apply(Action::MoveDown);
        }
    }

    #[test]
    fn linewise_increments_each_selected_line() {
        let mut e = editor_with("1\n1\n1");
        select_lines(&mut e, 0, 2);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines(), vec!["2", "2", "2"]);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn linewise_sequence_increments_progressively() {
        let mut e = editor_with("1\n1\n1");
        select_lines(&mut e, 0, 2);
        e.apply(Action::IncrementSequence);
        assert_eq!(e.text_lines(), vec!["2", "3", "4"]);
    }

    #[test]
    fn linewise_count_applies_to_each_line() {
        let mut e = editor_with("1\n1\n1");
        select_lines(&mut e, 0, 2);
        e.apply(Action::PushCount(3));
        e.apply(Action::Increment);
        assert_eq!(e.text_lines(), vec!["4", "4", "4"]);
    }

    #[test]
    fn linewise_sequence_count_progresses_by_count() {
        let mut e = editor_with("1\n1\n1");
        select_lines(&mut e, 0, 2);
        e.apply(Action::PushCount(3));
        e.apply(Action::IncrementSequence);
        assert_eq!(e.text_lines(), vec!["4", "7", "10"]);
    }

    #[test]
    fn charwise_increments_one_number_per_line() {
        let mut e = editor_with("1 2 3");
        e.apply(Action::EnterVisual);
        for _ in 0..6 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "2 2 3");
    }

    #[test]
    fn charwise_starts_at_selection_start() {
        let mut e = editor_with("1 2 3");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "1 3 3");
    }

    #[test]
    fn charwise_limits_number_to_selection() {
        let mut e = editor_with("1234");
        e.apply(Action::EnterVisual);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "2234");
        let mut e = editor_with("1234");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "1334");
        let mut e = editor_with("12345");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "12445");
    }

    #[test]
    fn block_increments_each_selected_line() {
        let mut e = editor_with("1 2\n3 4");
        e.apply(Action::EnterVisualBlock);
        e.apply(Action::MoveDown);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines(), vec!["2 2", "4 4"]);
    }

    #[test]
    fn block_respects_columns() {
        let mut e = editor_with("123 45\n678 90");
        e.apply(Action::EnterVisualBlock);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines(), vec!["124 45", "679 90"]);
    }

    #[test]
    fn visual_hex_limited_by_selection() {
        let mut e = editor_with("0x1f");
        e.apply(Action::EnterVisual);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "1x1f");
        let mut e = editor_with("0x1f");
        e.apply(Action::EnterVisual);
        for _ in 0..3 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "0x20");
    }

    #[test]
    fn visual_negative_sign_inside_selection() {
        let mut e = editor_with("-5");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "-4");
        let mut e = editor_with("-5");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines()[0], "-6");
    }

    #[test]
    fn visual_without_number_is_noop() {
        let mut e = editor_with("abc\ndef");
        select_lines(&mut e, 0, 1);
        e.apply(Action::Increment);
        assert_eq!(e.text_lines(), vec!["abc", "def"]);
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn visual_undo_reverts_all_lines() {
        let mut e = editor_with("1\n1");
        select_lines(&mut e, 0, 1);
        e.apply(Action::Increment);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines(), vec!["1", "1"]);
    }

    #[test]
    fn visual_repeat_uses_translated_area() {
        let mut e = editor_with("1\n1\n1\n2\n2\n2");
        select_lines(&mut e, 0, 2);
        e.apply(Action::Increment);
        e.apply(Action::JumpTo(2, 0));
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines(), vec!["2", "2", "3", "3", "3", "2"]);
    }

    #[test]
    fn visual_repeat_keeps_count() {
        let mut e = editor_with("1\n1\n1\n2\n2\n2");
        select_lines(&mut e, 0, 2);
        e.apply(Action::PushCount(3));
        e.apply(Action::Increment);
        e.apply(Action::JumpTo(2, 0));
        e.apply(Action::RepeatLastChange);
        assert_eq!(e.text_lines(), vec!["4", "4", "7", "5", "5", "2"]);
    }
}
