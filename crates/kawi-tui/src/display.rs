use kawi_buffer::unicode;

fn cell_width(grapheme: &str, screen: usize, tabstop: usize) -> usize {
    unicode::cluster_width(grapheme, screen, tabstop)
}

fn is_displayable(grapheme: &str) -> bool {
    grapheme == "\t" || !grapheme.chars().any(char::is_control)
}

pub fn screen_col(text: &str, chars: usize, tabstop: usize) -> usize {
    let mut screen: usize = 0;
    for grapheme in unicode::graphemes(text) {
        if grapheme.start() >= chars {
            break;
        }
        if is_displayable(grapheme.text()) {
            screen = screen.saturating_add(cell_width(grapheme.text(), screen, tabstop));
        }
    }
    screen
}

pub fn screen_len(text: &str, tabstop: usize) -> usize {
    screen_col(text, unicode::char_count(text), tabstop)
}

pub fn slice_screen(text: &str, from: usize, to: usize, tabstop: usize) -> String {
    let mut out = String::new();
    slice_screen_into(text, from, to, tabstop, &mut out);
    out
}

pub fn slice_screen_into(
    text: &str,
    from: usize,
    to: usize,
    tabstop: usize,
    out: &mut String,
) -> usize {
    out.clear();
    if from >= to {
        return 0;
    }
    let mut visible: usize = 0;
    let mut screen: usize = 0;
    for grapheme in unicode::graphemes(text) {
        if !is_displayable(grapheme.text()) {
            continue;
        }
        let width = cell_width(grapheme.text(), screen, tabstop);
        let start = screen;
        let end = screen.saturating_add(width);
        screen = end;
        if end <= from {
            continue;
        }
        if start >= to {
            break;
        }
        if grapheme.text() == "\t" {
            let count = end.min(to).saturating_sub(start.max(from));
            push_spaces(out, count);
            visible = visible.saturating_add(count);
        } else if start >= from && end <= to {
            out.push_str(grapheme.text());
            visible = visible.saturating_add(width);
        }
    }
    visible
}

fn push_spaces(out: &mut String, count: usize) {
    out.extend(std::iter::repeat_n(' ', count));
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";

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
    fn screen_col_counts_grapheme_clusters() {
        assert_eq!(screen_col(FAMILY, 1, 8), 2);
        assert_eq!(screen_col(FAMILY, 7, 8), 2);
        assert_eq!(screen_col(&format!("a{FAMILY}b"), 8, 8), 3);
    }

    #[test]
    fn screen_len_counts_cells() {
        assert_eq!(screen_len("\tx", 8), 9);
        assert_eq!(screen_len("", 8), 0);
        assert_eq!(screen_len("éx", 8), 2);
        assert_eq!(screen_len(FAMILY, 8), 2);
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
    fn slice_screen_keeps_clusters_intact() {
        let line = format!("a{FAMILY}b");
        assert_eq!(
            slice_screen(&line, 0, 3, 8),
            "a\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}"
        );
        assert_eq!(slice_screen(&line, 0, 2, 8), "a");
        assert_eq!(slice_screen(&line, 3, 4, 8), "b");
        assert_eq!(slice_screen(&line, 1, 2, 8), "");
    }

    #[test]
    fn slice_screen_into_reports_width_and_reuses_buffer() {
        let mut buffer = String::from("stale");
        let visible = slice_screen_into("\tx", 0, 20, 8, &mut buffer);
        assert_eq!(buffer, "        x");
        assert_eq!(visible, 9);
        let visible = slice_screen_into("a\tb", 1, 9, 8, &mut buffer);
        assert_eq!(buffer, "       b");
        assert_eq!(visible, 8);
        let visible = slice_screen_into("abc", 5, 8, 8, &mut buffer);
        assert!(buffer.is_empty());
        assert_eq!(visible, 0);
    }

    #[test]
    fn slice_screen_omits_control_characters() {
        assert_eq!(slice_screen("a\u{1b}[31mb\u{7f}\r", 0, 32, 8), "a[31mb");
        assert_eq!(slice_screen("a\tb\u{9b}c", 0, 32, 8), "a       bc");
    }
}
