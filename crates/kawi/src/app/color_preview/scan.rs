use crossterm::style::Color;
use kawi_tui::frame::DecorationSpan;

use super::color::HexColor;

const HASH: u8 = b'#';
const HASH_CHAR: char = '#';
const HEX_DIGIT_MAX: usize = 8;

pub(super) fn spans(lines: &[String], rows: &[usize], background: Color) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();
    for (line, row) in lines.iter().zip(rows.iter().copied()) {
        scan_line(row, line, background, &mut spans);
    }
    spans
}

fn scan_line(row: usize, line: &str, background: Color, spans: &mut Vec<DecorationSpan>) {
    let bytes = line.as_bytes();
    if !bytes.contains(&HASH) {
        return;
    }
    let mut column = 0;
    for (index, character) in line.char_indices() {
        if character == HASH_CHAR {
            if let Some((color, token_len)) = color_at(bytes, index) {
                spans.push(DecorationSpan {
                    row,
                    start: column,
                    end: column.saturating_add(token_len),
                    style: color.to_style(background),
                });
            }
        }
        column = column.saturating_add(1);
    }
}

fn color_at(bytes: &[u8], index: usize) -> Option<(HexColor, usize)> {
    if bytes.get(index) != Some(&HASH) || !is_boundary_before(bytes, index) {
        return None;
    }
    let digits_start = index.saturating_add(1);
    let digits_len = hex_digits(bytes, digits_start);
    let token_len = digits_len.saturating_add(1);
    if !is_boundary_after(bytes, digits_start.saturating_add(digits_len)) {
        return None;
    }
    let digits = bytes.get(digits_start..digits_start.saturating_add(digits_len))?;
    Some((HexColor::from_digits(digits)?, token_len))
}

fn hex_digits(bytes: &[u8], start: usize) -> usize {
    bytes
        .get(start..)
        .unwrap_or_default()
        .iter()
        .take(HEX_DIGIT_MAX)
        .take_while(|byte| byte.is_ascii_hexdigit())
        .count()
}

fn is_boundary_before(bytes: &[u8], index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|previous| bytes.get(previous))
        .is_none_or(|byte| is_boundary(*byte))
}

fn is_boundary_after(bytes: &[u8], index: usize) -> bool {
    bytes.get(index).is_none_or(|byte| is_boundary(*byte))
}

fn is_boundary(byte: u8) -> bool {
    !byte.is_ascii_alphanumeric() && byte != b'_' && byte != b'-' && byte != HASH && byte != b'\\'
}

#[cfg(test)]
mod tests {
    use crossterm::style::Color;
    use kawi_tui::frame::PluginStyle;

    use super::{color_at, spans};

    fn background() -> Color {
        Color::Rgb {
            r: 18,
            g: 18,
            b: 18,
        }
    }

    fn style_of(line: &str) -> PluginStyle {
        let lines = vec![line.to_string()];
        spans(&lines, &[0], background())[0].style
    }

    #[test]
    fn finds_short_and_long_hex_colors() {
        assert_eq!(color_at(b"#abc", 0).map(|(_, len)| len), Some(4));
        assert_eq!(color_at(b"#abcd", 0).map(|(_, len)| len), Some(5));
        assert_eq!(color_at(b"#aabbcc", 0).map(|(_, len)| len), Some(7));
        assert_eq!(color_at(b"#aabbccdd", 0).map(|(_, len)| len), Some(9));
    }

    #[test]
    fn rejects_invalid_lengths() {
        for value in ["#ab", "#abcde", "#abcdefa", "#abcdefghi"] {
            assert!(color_at(value.as_bytes(), 0).is_none(), "{value}");
        }
    }

    #[test]
    fn rejects_tokens_with_identifier_boundaries() {
        for value in ["foo#abc", "#abc-def", "#abc_xyz", "##abc", "\\#abc"] {
            assert!(color_at(value.as_bytes(), value.find('#').unwrap_or_default()).is_none());
        }
    }

    #[test]
    fn accepts_uppercase_and_mixed_case_digits() {
        assert!(color_at(b"#AaBbCc", 0).is_some());
    }

    #[test]
    fn skips_tokens_inside_identifiers() {
        assert_eq!(color_at(b"x#abc", 1), None);
        assert_eq!(color_at(b"#abcx", 0), None);
    }

    #[test]
    fn computes_readable_foreground() {
        let dark = style_of("#000000");
        assert_eq!(
            dark.foreground,
            Color::Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        let light = style_of("#ffffff");
        assert_eq!(light.foreground, Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn blends_alpha_over_the_theme_background() {
        let style = style_of("#00000000");
        assert_eq!(style.background, background());
        let style = style_of("#ff000080");
        assert_eq!(style.background, Color::Rgb { r: 137, g: 9, b: 9 });
    }

    #[test]
    fn generates_spans_with_character_columns() {
        let lines = vec!["é #fff".to_string()];
        let found = spans(&lines, &[7], background());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].row, 7);
        assert_eq!(found[0].start, 2);
        assert_eq!(found[0].end, 6);
    }

    #[test]
    fn ignores_lines_without_colors() {
        let lines = vec!["plain text".to_string()];
        assert!(spans(&lines, &[0], background()).is_empty());
    }
}
