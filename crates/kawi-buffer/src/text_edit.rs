use std::fmt;

use ropey::Rope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEditError {
    InvalidRange,
    Overlapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionError {
    LineOutOfRange { line: usize },
    ColumnOutOfRange { byte_column: usize },
    NotCharBoundary { byte_column: usize },
}

impl fmt::Display for PositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineOutOfRange { line } => write!(formatter, "line {line} is out of range"),
            Self::ColumnOutOfRange { byte_column } => {
                write!(formatter, "byte column {byte_column} is out of range")
            }
            Self::NotCharBoundary { byte_column } => {
                write!(
                    formatter,
                    "byte column {byte_column} is not a UTF-8 boundary"
                )
            }
        }
    }
}

pub fn char_offset_at(
    text: &Rope,
    line: usize,
    byte_column: usize,
) -> Result<usize, PositionError> {
    if line >= text.len_lines() {
        return Err(PositionError::LineOutOfRange { line });
    }
    let line_text = text.line(line);
    if byte_column > line_text.len_bytes() {
        return Err(PositionError::ColumnOutOfRange { byte_column });
    }
    let character = line_text.byte_to_char(byte_column);
    if line_text.char_to_byte(character) != byte_column {
        return Err(PositionError::NotCharBoundary { byte_column });
    }
    Ok(text.line_to_char(line) + character)
}

fn ordered_edits(edits: &[TextEdit]) -> Result<Vec<&TextEdit>, TextEditError> {
    let mut ordered: Vec<&TextEdit> = edits.iter().collect();
    ordered.sort_by_key(|edit| (edit.start, edit.end));
    let mut last_end = 0;
    for edit in &ordered {
        if edit.start > edit.end {
            return Err(TextEditError::InvalidRange);
        }
        if edit.start < last_end {
            return Err(TextEditError::Overlapping);
        }
        last_end = edit.end;
    }
    Ok(ordered)
}

pub fn validate_text_edits(text_len: usize, edits: &[TextEdit]) -> Result<(), TextEditError> {
    for edit in ordered_edits(edits)? {
        if edit.end > text_len {
            return Err(TextEditError::InvalidRange);
        }
    }
    Ok(())
}

pub fn apply_text_edits(text: &Rope, edits: &[TextEdit]) -> Result<Rope, TextEditError> {
    validate_text_edits(text.len_chars(), edits)?;
    let mut result = Rope::new();
    let mut cursor = 0;
    for edit in ordered_edits(edits)? {
        result.append(text.slice(cursor..edit.start).into());
        result.append(Rope::from_str(&edit.text));
        cursor = edit.end;
    }
    result.append(text.slice(cursor..text.len_chars()).into());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(start: usize, end: usize, text: &str) -> TextEdit {
        TextEdit {
            start,
            end,
            text: text.to_string(),
        }
    }

    #[test]
    fn applies_insertions_replacements_and_deletions() {
        let text = Rope::from_str("one two three");
        let edits = vec![edit(0, 3, "1"), edit(4, 7, "2"), edit(8, 13, "")];
        let result = apply_text_edits(&text, &edits).expect("edits");
        assert_eq!(result.to_string(), "1 2 ");
    }

    #[test]
    fn applies_edits_in_any_input_order() {
        let text = Rope::from_str("one two three");
        let edits = vec![edit(8, 13, ""), edit(0, 3, "1"), edit(4, 7, "2")];
        let result = apply_text_edits(&text, &edits).expect("edits");
        assert_eq!(result.to_string(), "1 2 ");
    }

    #[test]
    fn rejects_invalid_and_overlapping_ranges() {
        let text = Rope::from_str("abc");
        assert_eq!(
            apply_text_edits(&text, &[edit(1, 4, "x")]),
            Err(TextEditError::InvalidRange)
        );
        assert_eq!(
            apply_text_edits(&text, &[edit(1, 3, "x"), edit(0, 2, "y")]),
            Err(TextEditError::Overlapping)
        );
    }

    #[test]
    fn resolves_byte_columns_to_char_offsets() {
        let text = Rope::from_str("é\nx");
        assert_eq!(char_offset_at(&text, 0, 0), Ok(0));
        assert_eq!(char_offset_at(&text, 0, 2), Ok(1));
        assert_eq!(char_offset_at(&text, 1, 0), Ok(2));
        assert_eq!(
            char_offset_at(&text, 0, 1),
            Err(PositionError::NotCharBoundary { byte_column: 1 })
        );
        assert_eq!(
            char_offset_at(&text, 2, 0),
            Err(PositionError::LineOutOfRange { line: 2 })
        );
        assert_eq!(
            char_offset_at(&text, 0, 4),
            Err(PositionError::ColumnOutOfRange { byte_column: 4 })
        );
    }

    #[test]
    fn permits_adjacent_zero_width_edits() {
        let text = Rope::from_str("ab");
        let edits = vec![edit(1, 1, "x"), edit(1, 1, "y")];
        let result = apply_text_edits(&text, &edits).expect("edits");
        assert_eq!(result.to_string(), "axyb");
    }
}
