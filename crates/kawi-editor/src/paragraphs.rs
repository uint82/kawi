use kawi_buffer::Buffer;

const PARAGRAPHS: &str = "IPLPPPQPP TPHPLIPpLpItpplpipbp";
const SECTIONS: &str = "SHNHH HUnhsh";

pub(crate) fn is_paragraph_start(buffer: &Buffer, row: usize) -> bool {
    let Some(line) = buffer.line_cow(row) else {
        return false;
    };
    match line.as_bytes().first() {
        None => true,
        Some(b'\x0c') => true,
        Some(b'.') => in_macro(PARAGRAPHS, &line[1..]) || in_macro(SECTIONS, &line[1..]),
        Some(_) => false,
    }
}

fn in_macro(option: &str, text: &str) -> bool {
    let option = option.as_bytes();
    let text = text.as_bytes();
    let first = text.first().copied().unwrap_or(0);
    let second = text.get(1).copied().unwrap_or(0);
    let mut index = 0;
    while index < option.len() {
        let opt_first = option[index];
        let opt_second = option.get(index + 1).copied().unwrap_or(0);
        let first_matches =
            opt_first == first || (opt_first == b' ' && (first == 0 || first == b' '));
        let second_matches = opt_second == second
            || ((opt_second == 0 || opt_second == b' ')
                && (first == 0 || second == 0 || second == b' '));
        if first_matches && second_matches {
            return true;
        }
        index += 2;
        if index >= option.len() {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    #[test]
    fn empty_lines_start_paragraphs() {
        let b = buf("text\n\nmore");
        assert!(is_paragraph_start(&b, 1));
        assert!(!is_paragraph_start(&b, 0));
    }

    #[test]
    fn form_feed_starts_paragraphs() {
        let b = buf("\x0ctext");
        assert!(is_paragraph_start(&b, 0));
    }

    #[test]
    fn paragraph_macros_start_paragraphs() {
        let b = buf(".PP\n.IP tag\n.P\n.LP\n.Pp\n.TP");
        for row in 0..6 {
            assert!(is_paragraph_start(&b, row), "row {row}");
        }
    }

    #[test]
    fn section_macros_start_paragraphs() {
        let b = buf(".SH NAME\n.NH\n.HU\n.nh\n.sh");
        for row in 0..5 {
            assert!(is_paragraph_start(&b, row), "row {row}");
        }
    }

    #[test]
    fn non_macro_dot_lines_do_not_start() {
        let b = buf(".Pfoo\n...\n.foo\n.p\n. P\nx.PP");
        for row in 0..6 {
            assert!(!is_paragraph_start(&b, row), "row {row}");
        }
    }

    #[test]
    fn macro_match_uses_two_characters() {
        let b = buf(".PPx rest");
        assert!(is_paragraph_start(&b, 0));
    }

    #[test]
    fn plain_text_does_not_start() {
        let b = buf("text\nIP\nPP");
        for row in 0..3 {
            assert!(!is_paragraph_start(&b, row), "row {row}");
        }
    }
}
