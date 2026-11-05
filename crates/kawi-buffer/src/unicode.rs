use unicode_segmentation::UnicodeSegmentation;

mod cell;
mod width;

pub use cell::{render_cluster, RenderedCluster};
pub use width::{cluster_width, display_width, needs_terminal_reposition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grapheme<'a> {
    text: &'a str,
    start: usize,
    end: usize,
    start_byte: usize,
    end_byte: usize,
}

impl<'a> Grapheme<'a> {
    pub fn text(self) -> &'a str {
        self.text
    }

    pub fn start(self) -> usize {
        self.start
    }

    pub fn end(self) -> usize {
        self.end
    }

    pub fn start_byte(self) -> usize {
        self.start_byte
    }

    pub fn end_byte(self) -> usize {
        self.end_byte
    }
}

pub struct Graphemes<'a> {
    remaining: &'a str,
    char_index: usize,
    byte_index: usize,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = Grapheme<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let text = self.remaining.graphemes(true).next()?;
        let start = self.char_index;
        let end = start.saturating_add(text.chars().count());
        let start_byte = self.byte_index;
        let end_byte = start_byte.saturating_add(text.len());
        self.remaining = &self.remaining[text.len()..];
        self.char_index = end;
        self.byte_index = end_byte;
        Some(Grapheme {
            text,
            start,
            end,
            start_byte,
            end_byte,
        })
    }
}

pub fn graphemes(text: &str) -> Graphemes<'_> {
    Graphemes {
        remaining: text,
        char_index: 0,
        byte_index: 0,
    }
}

pub fn char_count(text: &str) -> usize {
    text.chars().count()
}

pub fn grapheme_at(text: &str, index: usize) -> Option<Grapheme<'_>> {
    graphemes(text).find(|grapheme| index < grapheme.end())
}

pub fn cluster_start(text: &str, index: usize) -> usize {
    grapheme_at(text, index).map_or_else(|| char_count(text), Grapheme::start)
}

pub fn cluster_end(text: &str, index: usize) -> usize {
    grapheme_at(text, index).map_or_else(|| char_count(text), Grapheme::end)
}

pub fn snap_end(text: &str, index: usize) -> usize {
    match grapheme_at(text, index) {
        Some(grapheme) if grapheme.start() < index => grapheme.end(),
        _ => index.min(char_count(text)),
    }
}

pub fn prev_boundary(text: &str, index: usize) -> usize {
    let mut boundary = 0;
    for grapheme in graphemes(text) {
        if grapheme.start() >= index {
            break;
        }
        boundary = grapheme.start();
    }
    boundary
}

pub fn next_boundary(text: &str, index: usize) -> usize {
    for grapheme in graphemes(text) {
        if grapheme.start() > index {
            return grapheme.start();
        }
    }
    char_count(text)
}

pub fn last_boundary(text: &str) -> usize {
    graphemes(text)
        .last()
        .map_or(0, |grapheme| grapheme.start())
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

    fn widths(text: &str) -> Vec<(usize, usize, usize)> {
        graphemes(text)
            .map(|grapheme| {
                (
                    grapheme.start(),
                    grapheme.end(),
                    display_width(grapheme.text()),
                )
            })
            .collect()
    }

    #[test]
    fn counts_chars_and_graphemes_separately() {
        assert_eq!(char_count(FAMILY), 7);
        assert_eq!(graphemes(FAMILY).count(), 1);
        assert_eq!(char_count(FLAG), 2);
        assert_eq!(graphemes(FLAG).count(), 1);
    }

    #[test]
    fn measures_cluster_widths() {
        assert_eq!(widths(FAMILY), [(0, 7, 2)]);
        assert_eq!(widths(FLAG), [(0, 2, 2)]);
        assert_eq!(widths(SKIN), [(0, 2, 2)]);
        assert_eq!(widths(ACCENT), [(0, 2, 1)]);
        assert_eq!(widths(CONJUNCT), [(0, 3, 1)]);
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
        assert!(!needs_terminal_reposition(" \u{301}"));
    }

    #[test]
    fn matches_neovim_emoji_widths() {
        assert_eq!(display_width(KEYCAP_ONE), 1);
        assert_eq!(display_width("\u{1f577}"), 2);
        assert_eq!(display_width("\u{1f441}"), 2);
        assert_eq!(display_width("\u{1f170}"), 1);
        assert_eq!(display_width("\u{1f170}\u{fe0f}"), 2);
    }

    #[test]
    fn finds_the_cluster_containing_an_index() {
        let line = format!("a{FAMILY}b");
        let family = grapheme_at(&line, 1).expect("family");
        assert_eq!((family.start(), family.end()), (1, 8));
        assert_eq!(grapheme_at(&line, 7).map(Grapheme::start), Some(1));
        assert_eq!(grapheme_at(&line, 8).map(Grapheme::start), Some(8));
        assert_eq!(grapheme_at(&line, 9).map(Grapheme::start), None);
    }

    #[test]
    fn snaps_to_cluster_boundaries() {
        let line = format!("a{FAMILY}b");
        assert_eq!(cluster_start(&line, 3), 1);
        assert_eq!(cluster_end(&line, 3), 8);
        assert_eq!(cluster_start(&line, 9), 9);
        assert_eq!(cluster_end(&line, 9), 9);
        assert_eq!(snap_end(&line, 3), 8);
        assert_eq!(snap_end(&line, 8), 8);
        assert_eq!(snap_end(&line, 9), 9);
    }

    #[test]
    fn steps_between_cluster_boundaries() {
        let line = format!("a{FAMILY}b");
        assert_eq!(prev_boundary(&line, 8), 1);
        assert_eq!(prev_boundary(&line, 9), 8);
        assert_eq!(next_boundary(&line, 0), 1);
        assert_eq!(next_boundary(&line, 1), 8);
        assert_eq!(next_boundary(&line, 8), 9);
        assert_eq!(next_boundary(&line, 9), 9);
        assert_eq!(last_boundary(&line), 8);
    }

    #[test]
    fn expands_tabs_from_the_current_screen_column() {
        assert_eq!(cluster_width("\t", 0, 8), 8);
        assert_eq!(cluster_width("\t", 3, 8), 5);
        assert_eq!(cluster_width("\t", 3, 0), 1);
        assert_eq!(cluster_width("a", 3, 8), 1);
        assert_eq!(cluster_width(FAMILY, 0, 8), 2);
    }

    #[test]
    fn empty_text_has_zero_boundaries() {
        assert_eq!(graphemes("").count(), 0);
        assert_eq!(prev_boundary("", 0), 0);
        assert_eq!(next_boundary("", 0), 0);
        assert_eq!(last_boundary(""), 0);
        assert_eq!(display_width(""), 0);
    }
}
