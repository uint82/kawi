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
    fn permits_adjacent_zero_width_edits() {
        let text = Rope::from_str("ab");
        let edits = vec![edit(1, 1, "x"), edit(1, 1, "y")];
        let result = apply_text_edits(&text, &edits).expect("edits");
        assert_eq!(result.to_string(), "axyb");
    }
}
