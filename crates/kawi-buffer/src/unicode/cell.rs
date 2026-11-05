use std::borrow::Cow;

use unicode_width::UnicodeWidthChar;

use super::graphemes;
use super::width::{printable_cluster_width, tab_width, MINIMUM_CELL_WIDTH};

const HEX_BASE_WIDTH: usize = 2;
const SPACE_BYTES: usize = 1;
const ASCII_CONTROL_WIDTH: usize = 2;
const BYTE_PLACEHOLDER_WIDTH: usize = 4;
const WIDE_PLACEHOLDER_WIDTH: usize = 6;
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedCluster<'a> {
    kind: ClusterKind<'a>,
    width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClusterKind<'a> {
    Raw(&'a str),
    Leading(&'a str),
    Spaces(usize),
    Placeholder(Placeholder),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placeholder {
    Caret(u8),
    Hex { code: u32, width: usize },
}

impl<'a> RenderedCluster<'a> {
    pub fn width(self) -> usize {
        self.width
    }

    pub fn is_placeholder(self) -> bool {
        matches!(self.kind, ClusterKind::Placeholder(_))
    }

    pub fn text(self) -> Cow<'a, str> {
        match self.kind {
            ClusterKind::Raw(text) => Cow::Borrowed(text),
            _ => {
                let mut out = String::new();
                self.write_to(&mut out);
                Cow::Owned(out)
            }
        }
    }

    pub fn write_to(self, out: &mut String) {
        match self.kind {
            ClusterKind::Raw(text) => out.push_str(text),
            ClusterKind::Leading(text) => {
                out.push(' ');
                out.push_str(text);
            }
            ClusterKind::Spaces(count) => push_spaces(out, count),
            ClusterKind::Placeholder(placeholder) => placeholder.write_to(out),
        }
    }
}

impl Placeholder {
    fn width(self) -> usize {
        match self {
            Self::Caret(_) => ASCII_CONTROL_WIDTH,
            Self::Hex { width, .. } => width,
        }
    }

    fn write_to(self, out: &mut String) {
        match self {
            Self::Caret(byte) => {
                out.push('^');
                out.push(char::from(byte ^ 0x40));
            }
            Self::Hex { code, width } => {
                out.push('<');
                push_hex(out, code, width - HEX_BASE_WIDTH);
                out.push('>');
            }
        }
    }
}

pub fn render_cluster(text: &str, screen: usize, tabstop: usize) -> RenderedCluster<'_> {
    let Some(first) = text.chars().next() else {
        return RenderedCluster {
            kind: ClusterKind::Raw(text),
            width: 0,
        };
    };
    if text == "\t" {
        let width = tab_width(screen, tabstop);
        return RenderedCluster {
            kind: ClusterKind::Spaces(width),
            width,
        };
    }
    if let Some(placeholder) = placeholder(first) {
        return RenderedCluster {
            kind: ClusterKind::Placeholder(placeholder),
            width: placeholder.width(),
        };
    }
    if first.is_ascii() {
        return RenderedCluster {
            kind: ClusterKind::Raw(text),
            width: MINIMUM_CELL_WIDTH,
        };
    }
    let width = printable_cluster_width(text, first);
    let kind = if is_composing_first(first) {
        ClusterKind::Leading(text)
    } else {
        ClusterKind::Raw(text)
    };
    RenderedCluster { kind, width }
}

pub(super) fn placeholder_width(first: char) -> Option<usize> {
    placeholder(first).map(Placeholder::width)
}

fn placeholder(first: char) -> Option<Placeholder> {
    let code = first as u32;
    if first.is_ascii_control() {
        if first == '\t' {
            return None;
        }
        return Some(Placeholder::Caret(first as u8));
    }
    if (0x80..=0x9f).contains(&code) {
        return Some(Placeholder::Hex {
            code,
            width: BYTE_PLACEHOLDER_WIDTH,
        });
    }
    if is_nonprintable(code) {
        let width = if code > 0xff {
            WIDE_PLACEHOLDER_WIDTH
        } else {
            BYTE_PLACEHOLDER_WIDTH
        };
        return Some(Placeholder::Hex { code, width });
    }
    None
}

fn is_nonprintable(code: u32) -> bool {
    matches!(
        code,
        0x070f
            | 0x180b..=0x180e
            | 0x200b..=0x200f
            | 0x202a..=0x202e
            | 0x2060..=0x206f
            | 0xd800..=0xdfff
            | 0xfeff
            | 0xfff9..=0xfffb
            | 0xfffe..=0xffff
    )
}

fn is_composing_first(first: char) -> bool {
    if first.is_ascii() || !may_compose_with_space(first) {
        return false;
    }
    let mut buffer = [0_u8; 8];
    buffer[0] = b' ';
    let len = SPACE_BYTES + first.encode_utf8(&mut buffer[SPACE_BYTES..]).len();
    std::str::from_utf8(&buffer[..len]).is_ok_and(|text| graphemes(text).count() == 1)
}

fn may_compose_with_space(first: char) -> bool {
    let code = first as u32;
    matches!(
        code,
        0x1f3fb..=0x1f3ff | 0xfe00..=0xfe0f | 0xe0100..=0xe01ef
    ) || UnicodeWidthChar::width(first) == Some(0)
}

fn push_spaces(out: &mut String, count: usize) {
    out.extend(std::iter::repeat_n(' ', count));
}

fn push_hex(out: &mut String, code: u32, digits: usize) {
    for shift in (0..digits).rev() {
        let nibble = (code >> (shift * 4)) & 0xf;
        out.push(char::from(HEX_DIGITS[nibble as usize]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unicode::cluster_width;

    fn rendered(text: &str) -> String {
        let mut out = String::new();
        render_cluster(text, 0, 8).write_to(&mut out);
        out
    }

    #[test]
    fn renders_nonprintable_cells_like_neovim() {
        assert_eq!(rendered("\u{202e}"), "<202e>");
        assert_eq!(rendered("\u{200b}"), "<200b>");
        assert_eq!(rendered("\u{feff}"), "<feff>");
        assert_eq!(rendered("\u{85}"), "<85>");
        assert_eq!(rendered("\u{1b}"), "^[");
        assert_eq!(rendered("\u{7f}"), "^?");
    }

    #[test]
    fn identifies_placeholder_clusters() {
        assert!(render_cluster("\u{feff}", 0, 8).is_placeholder());
        assert!(render_cluster("\u{1b}", 0, 8).is_placeholder());
        assert!(!render_cluster("a", 0, 8).is_placeholder());
        assert!(!render_cluster("\u{ad}", 0, 8).is_placeholder());
    }

    #[test]
    fn keeps_composing_and_soft_hyphen_cells_visible() {
        assert_eq!(cluster_width("\u{ad}", 0, 8), 1);
        assert_eq!(rendered("\u{ad}"), "\u{ad}");
        assert_eq!(rendered("\u{301}"), " \u{301}");
        assert_eq!(cluster_width("\u{301}", 0, 8), 1);
    }

    #[test]
    fn expands_tabs_from_the_current_screen_column() {
        assert_eq!(rendered("\t"), "        ");
    }
}
