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
    kawi_buffer::unicode::display_width(text)
}

pub fn char_count(text: &str) -> usize {
    kawi_buffer::unicode::char_count(text)
}

pub fn truncate_to_display_width(text: &str, width: usize) -> &str {
    let mut used: usize = 0;
    for grapheme in kawi_buffer::unicode::graphemes(text) {
        let next = used.saturating_add(display_width(grapheme.text()));
        if next > width {
            return &text[..grapheme.start_byte()];
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
    fn counts_characters_not_bytes_or_cells() {
        assert_eq!(char_count("a\u{4e2d}\u{301}"), 3);
    }

    #[test]
    fn measures_emoji_sequences_as_single_cells() {
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
        let flag = "\u{1f1ee}\u{1f1e9}";
        let skin = "\u{1f44d}\u{1f3fd}";
        assert_eq!(display_width(family), 2);
        assert_eq!(display_width(flag), 2);
        assert_eq!(display_width(skin), 2);
        assert_eq!(display_width("e\u{301}"), 1);
    }

    #[test]
    fn truncates_emoji_sequences_as_single_cells() {
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
        let line = format!("a{family}b");
        assert_eq!(truncate_to_display_width(&line, 2), "a");
        assert_eq!(truncate_to_display_width(&line, 3), format!("a{family}"));
    }

    #[test]
    fn chooses_the_matching_form() {
        assert_eq!(count_label(1, "match", "matches"), "1 match");
        assert_eq!(count_label(2, "file", "files"), "2 files");
        assert_eq!(count_label(0, "file", "files"), "0 files");
    }
}
