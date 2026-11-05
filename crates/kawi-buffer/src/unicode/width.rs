use unicode_width::UnicodeWidthChar;

use super::cell::placeholder_width;
use super::graphemes;

pub(super) const MINIMUM_CELL_WIDTH: usize = 1;
const TAB_DISPLAY_WIDTH: usize = 1;

pub fn display_width(text: &str) -> usize {
    if text
        .bytes()
        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        return text.len();
    }
    graphemes(text)
        .map(|grapheme| cluster_width(grapheme.text(), 0, TAB_DISPLAY_WIDTH))
        .sum()
}

pub fn cluster_width(text: &str, screen: usize, tabstop: usize) -> usize {
    if text == "\t" {
        return tab_width(screen, tabstop);
    }
    cluster_display_width(text)
}

fn cluster_display_width(text: &str) -> usize {
    let Some(first) = text.chars().next() else {
        return 0;
    };
    if let Some(width) = placeholder_width(first) {
        return width;
    }
    printable_cluster_width(text, first)
}

pub(super) fn printable_cluster_width(text: &str, first: char) -> usize {
    if first.is_ascii() {
        return MINIMUM_CELL_WIDTH;
    }
    let width = UnicodeWidthChar::width(first).unwrap_or(0);
    if width >= 2 {
        return width;
    }
    if is_wide_emoji_start(first) || follows_emoji_presentation(text, first) {
        return 2;
    }
    MINIMUM_CELL_WIDTH
}

pub(super) fn tab_width(screen: usize, tabstop: usize) -> usize {
    let tabstop = tabstop.max(1);
    tabstop - screen % tabstop
}

fn follows_emoji_presentation(text: &str, first: char) -> bool {
    text[first.len_utf8()..].starts_with('\u{fe0f}') && is_emojilike(first)
}

fn is_emojilike(first: char) -> bool {
    ('\u{1f1e6}'..='\u{1f1ff}').contains(&first) || is_extended_pictographic(first)
}

fn is_wide_emoji_start(first: char) -> bool {
    first >= '\u{1f000}' && !is_ambiguous_width(first) && is_emojilike(first)
}

fn is_ambiguous_width(character: char) -> bool {
    UnicodeWidthChar::width(character) == Some(1)
        && UnicodeWidthChar::width_cjk(character) == Some(2)
}

fn is_extended_pictographic(first: char) -> bool {
    let mut buffer = [0_u8; 11];
    let len = first.encode_utf8(&mut buffer).len();
    let zwj = "\u{200d}".as_bytes();
    let emoji = "\u{1f600}".as_bytes();
    buffer[len..len + zwj.len()].copy_from_slice(zwj);
    let end = len + zwj.len() + emoji.len();
    buffer[len + zwj.len()..end].copy_from_slice(emoji);
    std::str::from_utf8(&buffer[..end]).is_ok_and(|probe| graphemes(probe).count() == 1)
}

pub fn needs_terminal_reposition(text: &str) -> bool {
    let Some(first) = text.chars().next() else {
        return false;
    };
    if !first.is_ascii() && (is_ambiguous_width(first) || is_emojilike(first)) {
        return true;
    }
    if text[first.len_utf8()..].starts_with('\u{fe0f}') {
        return true;
    }
    renders_without_width(text)
}

fn renders_without_width(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| UnicodeWidthChar::width(character).unwrap_or(0) == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
    const FLAG: &str = "\u{1f1ee}\u{1f1e9}";
    const SKIN: &str = "\u{1f44d}\u{1f3fd}";
    const ACCENT: &str = "e\u{301}";
    const CONJUNCT: &str = "\u{915}\u{94d}\u{937}";
    const KEYCAP_ONE: &str = "1\u{fe0f}\u{20e3}";

    #[test]
    fn measures_cluster_widths_from_the_first_character() {
        assert_eq!(cluster_width(FAMILY, 0, 8), 2);
        assert_eq!(cluster_width(FLAG, 0, 8), 2);
        assert_eq!(cluster_width(SKIN, 0, 8), 2);
        assert_eq!(cluster_width(ACCENT, 0, 8), 1);
        assert_eq!(cluster_width(CONJUNCT, 0, 8), 1);
    }

    #[test]
    fn promotes_emoji_after_a_variation_selector() {
        assert_eq!(cluster_width("\u{2764}\u{fe0e}", 0, 8), 1);
        assert_eq!(cluster_width("\u{2764}\u{fe0f}", 0, 8), 2);
        assert_eq!(cluster_width("\u{1f170}", 0, 8), 1);
        assert_eq!(cluster_width("\u{1f170}\u{fe0f}", 0, 8), 2);
        assert_eq!(cluster_width(KEYCAP_ONE, 0, 8), 1);
    }

    #[test]
    fn counts_soft_hyphen_as_one_cell() {
        assert_eq!(cluster_width("\u{ad}", 0, 8), 1);
    }

    #[test]
    fn expands_tabs_from_the_current_screen_column() {
        assert_eq!(cluster_width("\t", 0, 8), 8);
        assert_eq!(cluster_width("\t", 3, 8), 5);
        assert_eq!(cluster_width("\t", 3, 0), 1);
    }

    #[test]
    fn marks_cells_whose_host_width_may_differ() {
        assert!(needs_terminal_reposition("\u{2764}\u{fe0f}"));
        assert!(needs_terminal_reposition(
            "\u{2764}\u{fe0f}\u{200d}\u{1f525}"
        ));
        assert!(needs_terminal_reposition("\u{1f1ee}\u{1f1e9}"));
        assert!(needs_terminal_reposition("1\u{fe0f}\u{20e3}"));
        assert!(needs_terminal_reposition("\u{2600}\u{fe0f}"));
        assert!(needs_terminal_reposition("\u{1f600}"));
        assert!(needs_terminal_reposition("\u{ad}"));
        assert!(needs_terminal_reposition("\u{61c}"));
        assert!(needs_terminal_reposition("\u{e0001}"));
        assert!(!needs_terminal_reposition("1"));
        assert!(!needs_terminal_reposition("\u{4e2d}"));
        assert!(!needs_terminal_reposition("e\u{301}"));
        assert!(!needs_terminal_reposition("<202e>"));
        assert!(!needs_terminal_reposition(" \u{301}"));
    }
}
