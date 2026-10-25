pub const TAB_WIDTH: usize = 8;

pub fn sanitize_char(character: char) -> char {
    if character.is_control() && character != '\t' {
        ' '
    } else {
        character
    }
}

pub fn char_width(character: char) -> usize {
    unicode_width::UnicodeWidthChar::width(character).unwrap_or(0)
}

pub fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

pub fn truncate_to_display_width(text: &str, width: usize) -> &str {
    let mut used: usize = 0;
    for (index, character) in text.char_indices() {
        let next = used.saturating_add(char_width(character));
        if next > width {
            return &text[..index];
        }
        used = next;
    }
    text
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
    fn sanitizes_control_characters_but_keeps_tabs() {
        assert_eq!(sanitize_text("a\tb\u{1b}"), "a\tb ");
    }

    #[test]
    fn measures_display_cells() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("\u{4e2d}\u{6587}"), 4);
        assert_eq!(char_width('\u{4e2d}'), 2);
    }

    #[test]
    fn truncates_at_display_boundaries() {
        assert_eq!(truncate_to_display_width("ab\u{4e2d}c", 3), "ab");
        assert_eq!(truncate_to_display_width("ab\u{4e2d}c", 4), "ab\u{4e2d}");
    }

    #[test]
    fn chooses_the_matching_form() {
        assert_eq!(count_label(1, "match", "matches"), "1 match");
        assert_eq!(count_label(2, "file", "files"), "2 files");
        assert_eq!(count_label(0, "file", "files"), "0 files");
    }
}
