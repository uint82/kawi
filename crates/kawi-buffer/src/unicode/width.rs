use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::graphemes;

pub fn display_width(text: &str) -> usize {
    if text.is_ascii() {
        return UnicodeWidthStr::width(text);
    }
    graphemes(text)
        .map(|grapheme| cluster_display_width(grapheme.text()))
        .sum()
}

pub fn cluster_width(text: &str, screen: usize, tabstop: usize) -> usize {
    if text != "\t" {
        return cluster_display_width(text);
    }
    let tabstop = tabstop.max(1);
    tabstop - screen % tabstop
}

fn cluster_display_width(text: &str) -> usize {
    let Some(first) = text.chars().next() else {
        return 0;
    };
    if first.is_ascii() {
        return UnicodeWidthChar::width(first).unwrap_or(0);
    }
    let width = UnicodeWidthStr::width(text);
    if width != 1 {
        return width;
    }
    if is_wide_emoji_start(first) {
        return 2;
    }
    width
}

pub fn needs_terminal_reposition(text: &str) -> bool {
    let Some(first) = text.chars().next() else {
        return false;
    };
    if !first.is_ascii() && (is_ambiguous_width(first) || is_emojilike(first)) {
        return true;
    }
    text[first.len_utf8()..].starts_with('\u{fe0f}')
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
