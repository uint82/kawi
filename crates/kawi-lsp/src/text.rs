use lsp_types::{Position, PositionEncodingKind};
use ropey::Rope;

pub(crate) fn position(
    rope: &Rope,
    char_index: usize,
    encoding: &PositionEncodingKind,
) -> Position {
    let index = char_index.min(rope.len_chars());
    let line = rope.char_to_line(index);
    let line_start = rope.line_to_char(line);
    let character = encoded_len(rope.slice(line_start..index).chars(), encoding);
    Position::new(u32::try_from(line).unwrap_or(u32::MAX), character)
}

pub(crate) fn line_character(
    rope: &Rope,
    line: u32,
    character: u32,
    encoding: &PositionEncodingKind,
) -> u32 {
    let line = clamped_line(rope, line);
    let start = rope.line_to_char(line);
    let end = line_end(rope, line);
    char_index(rope.slice(start..end).chars(), character, encoding)
}

pub(crate) fn char_index(
    chars: impl Iterator<Item = char>,
    character: u32,
    encoding: &PositionEncodingKind,
) -> u32 {
    let mut units = 0u32;
    let mut index = 0u32;
    for value in chars {
        if units >= character {
            return index;
        }
        units = units.saturating_add(encoded_char_len(value, encoding));
        index = index.saturating_add(1);
    }
    index
}

pub(crate) fn clamped_line(rope: &Rope, line: u32) -> usize {
    usize::try_from(line)
        .unwrap_or(usize::MAX)
        .min(rope.len_lines().saturating_sub(1))
}

fn line_end(rope: &Rope, line: usize) -> usize {
    let start = rope.line_to_char(line);
    let mut end = if line.saturating_add(1) < rope.len_lines() {
        rope.line_to_char(line.saturating_add(1))
    } else {
        rope.len_chars()
    };
    if end > start && rope.get_char(end.saturating_sub(1)) == Some('\n') {
        end = end.saturating_sub(1);
        if end > start && rope.get_char(end.saturating_sub(1)) == Some('\r') {
            end = end.saturating_sub(1);
        }
    }
    end
}

fn encoded_len(chars: impl Iterator<Item = char>, encoding: &PositionEncodingKind) -> u32 {
    chars.fold(0u32, |total, value| {
        total.saturating_add(encoded_char_len(value, encoding))
    })
}

fn encoded_char_len(value: char, encoding: &PositionEncodingKind) -> u32 {
    if encoding == &PositionEncodingKind::UTF8 {
        u32::try_from(value.len_utf8()).unwrap_or(u32::MAX)
    } else if encoding == &PositionEncodingKind::UTF32 {
        1
    } else {
        u32::try_from(value.len_utf16()).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(text: &str) -> Rope {
        Rope::from_str(text)
    }

    #[test]
    fn positions_use_the_negotiated_encoding() {
        let rope = rope("a𝄞b");
        assert_eq!(
            position(&rope, 2, &PositionEncodingKind::UTF16),
            Position::new(0, 3)
        );
        assert_eq!(
            position(&rope, 2, &PositionEncodingKind::UTF8),
            Position::new(0, 5)
        );
        assert_eq!(
            position(&rope, 2, &PositionEncodingKind::UTF32),
            Position::new(0, 2)
        );
    }

    #[test]
    fn positions_track_lines_and_ends() {
        let text = rope("a\nb\n");
        assert_eq!(
            position(&text, 2, &PositionEncodingKind::UTF16),
            Position::new(1, 0)
        );
        assert_eq!(
            position(&text, 4, &PositionEncodingKind::UTF16),
            Position::new(2, 0)
        );
        let empty = rope("");
        assert_eq!(
            position(&empty, 0, &PositionEncodingKind::UTF16),
            Position::new(0, 0)
        );
    }

    #[test]
    fn line_characters_are_clamped_and_ignore_line_endings() {
        let text = rope("ab\r\ncd");
        assert_eq!(
            line_character(&text, 0, 99, &PositionEncodingKind::UTF16),
            2
        );
        assert_eq!(line_character(&text, 1, 1, &PositionEncodingKind::UTF16), 1);
        assert_eq!(line_character(&text, 9, 1, &PositionEncodingKind::UTF16), 1);
    }

    #[test]
    fn character_indices_clamp_to_the_line() {
        assert_eq!(
            char_index("a𝄞b".chars(), 3, &PositionEncodingKind::UTF16),
            2
        );
        assert_eq!(char_index("a𝄞b".chars(), 3, &PositionEncodingKind::UTF8), 2);
        assert_eq!(
            char_index("a𝄞b".chars(), 2, &PositionEncodingKind::UTF32),
            2
        );
        assert_eq!(
            char_index("a𝄞b".chars(), 99, &PositionEncodingKind::UTF16),
            3
        );
    }
}
