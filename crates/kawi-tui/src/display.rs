use kawi_buffer::tab_width;

use crate::text::char_width;

fn cell_width(ch: char, screen: usize, tabstop: usize) -> usize {
    if ch == '\t' {
        tab_width(ch, screen, tabstop)
    } else {
        char_width(ch)
    }
}

fn is_displayable(ch: char) -> bool {
    ch == '\t' || !ch.is_control()
}

pub fn screen_col(text: &str, chars: usize, tabstop: usize) -> usize {
    let mut screen = 0;
    for ch in text.chars().take(chars) {
        screen += cell_width(ch, screen, tabstop);
    }
    screen
}

pub fn screen_len(text: &str, tabstop: usize) -> usize {
    screen_col(text, text.chars().count(), tabstop)
}

pub fn slice_screen(text: &str, from: usize, to: usize, tabstop: usize) -> String {
    if from >= to {
        return String::new();
    }
    let mut out = String::new();
    let mut screen = 0;
    for ch in text.chars() {
        if !is_displayable(ch) {
            continue;
        }
        let width = cell_width(ch, screen, tabstop);
        let start = screen;
        let end = screen + width;
        screen = end;
        if end <= from {
            continue;
        }
        if start >= to {
            break;
        }
        let visible_start = start.max(from);
        let visible_end = end.min(to);
        if ch == '\t' {
            out.push_str(&" ".repeat(visible_end - visible_start));
        } else if start >= from {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_col_expands_tabs() {
        assert_eq!(screen_col("\tx", 1, 8), 8);
        assert_eq!(screen_col("a\tb", 2, 8), 8);
        assert_eq!(screen_col("\tx", 2, 8), 9);
        assert_eq!(screen_col("\tx", 2, 4), 5);
    }

    #[test]
    fn screen_col_counts_wide_and_multibyte_cells() {
        assert_eq!(screen_col("界x", 1, 8), 2);
        assert_eq!(screen_col("é界x", 2, 8), 3);
        assert_eq!(screen_col("\té界x", 3, 8), 11);
    }

    #[test]
    fn screen_len_counts_cells() {
        assert_eq!(screen_len("\tx", 8), 9);
        assert_eq!(screen_len("", 8), 0);
        assert_eq!(screen_len("éx", 8), 2);
    }

    #[test]
    fn slice_screen_keeps_full_line() {
        assert_eq!(slice_screen("\tx", 0, 20, 8), "        x");
        assert_eq!(slice_screen("abc", 0, 20, 8), "abc");
    }

    #[test]
    fn slice_screen_crops_tabs() {
        assert_eq!(slice_screen("\tx", 4, 20, 8), "    x");
        assert_eq!(slice_screen("\tx", 9, 20, 8), "");
        assert_eq!(slice_screen("a\tb", 1, 9, 8), "       b");
    }

    #[test]
    fn slice_screen_crops_chars() {
        assert_eq!(slice_screen("abcdef", 2, 4, 8), "cd");
        assert_eq!(slice_screen("abc", 2, 10, 8), "c");
        assert_eq!(slice_screen("abc", 5, 8, 8), "");
        assert_eq!(slice_screen("", 0, 1, 8), "");
    }

    #[test]
    fn slice_screen_omits_control_characters() {
        assert_eq!(slice_screen("a\u{1b}[31mb\u{7f}\r", 0, 32, 8), "a[31mb");
        assert_eq!(slice_screen("a\tb\u{9b}c", 0, 32, 8), "a       bc");
    }
}
