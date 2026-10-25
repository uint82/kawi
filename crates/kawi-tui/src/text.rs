pub fn sanitize_char(character: char) -> char {
    if character.is_control() || character == '\t' {
        ' '
    } else {
        character
    }
}

pub fn sanitize_text(text: &str) -> String {
    text.chars().map(sanitize_char).collect()
}

pub fn count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_control_characters_and_tabs() {
        assert_eq!(sanitize_text("a\tb\u{1b}"), "a b ");
    }

    #[test]
    fn chooses_the_matching_form() {
        assert_eq!(count_label(1, "match", "matches"), "1 match");
        assert_eq!(count_label(2, "file", "files"), "2 files");
        assert_eq!(count_label(0, "file", "files"), "0 files");
    }
}
