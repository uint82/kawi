use kawi_buffer::Buffer;

use crate::words::{dec_pos, inc_pos};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuoteMode {
    Smart,
    Blind,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MatchConfig {
    pub open: char,
    pub close: char,
    pub quotes: QuoteMode,
    pub match_escaped: bool,
}

impl MatchConfig {
    pub(crate) fn smart(open: char, close: char, match_escaped: bool) -> Self {
        Self {
            open,
            close,
            quotes: QuoteMode::Smart,
            match_escaped,
        }
    }

    pub(crate) fn blind(open: char, close: char) -> Self {
        Self {
            open,
            close,
            quotes: QuoteMode::Blind,
            match_escaped: false,
        }
    }
}

pub(crate) fn find_open_backward(
    buffer: &Buffer,
    pos: (usize, usize),
    config: MatchConfig,
) -> Option<(usize, usize)> {
    let mut scan = Scan::new(buffer, pos, true, config.close, config.open, config);
    scan.run(buffer)
}

pub(crate) fn find_close_forward(
    buffer: &Buffer,
    pos: (usize, usize),
    config: MatchConfig,
) -> Option<(usize, usize)> {
    let mut scan = Scan::new(buffer, pos, false, config.open, config.close, config);
    scan.run(buffer)
}

pub(crate) fn find_block_open(
    buffer: &Buffer,
    pos: (usize, usize),
    config: MatchConfig,
) -> Option<(usize, usize)> {
    let mut scan = Scan::new(buffer, pos, false, config.close, config.open, config);
    scan.run(buffer)
}

pub(crate) fn starts_escaped(buffer: &Buffer, pos: (usize, usize)) -> bool {
    backslash_run_before(&line_chars(buffer, pos.0), pos.1) % 2 == 1
}

struct Scan {
    pos: (usize, usize),
    backwards: bool,
    initc: char,
    findc: char,
    quotes: QuoteMode,
    match_escaped: bool,
    depth: usize,
    line: Vec<char>,
    line_quotes: i32,
    inquote: bool,
    start_in_quotes: Option<bool>,
    at_start: i32,
}

impl Scan {
    fn new(
        buffer: &Buffer,
        pos: (usize, usize),
        backwards: bool,
        initc: char,
        findc: char,
        config: MatchConfig,
    ) -> Self {
        let start_in_quotes = match config.quotes {
            QuoteMode::Smart => None,
            QuoteMode::Blind => Some(false),
        };
        Self {
            pos,
            backwards,
            initc,
            findc,
            quotes: config.quotes,
            match_escaped: config.match_escaped,
            depth: 0,
            line: line_chars(buffer, pos.0),
            line_quotes: -1,
            inquote: false,
            start_in_quotes,
            at_start: -1,
        }
    }

    fn run(&mut self, buffer: &Buffer) -> Option<(usize, usize)> {
        loop {
            if !self.advance(buffer) {
                return None;
            }
            if self.quotes == QuoteMode::Smart {
                self.update_line_quotes(buffer);
                if self.start_in_quotes.is_none() {
                    self.start_in_quotes = Some(false);
                }
            }
            if let Some(found) = self.process() {
                return Some(found);
            }
        }
    }

    fn advance(&mut self, buffer: &Buffer) -> bool {
        let (next, step) = if self.backwards {
            dec_pos(buffer, self.pos.0, self.pos.1)
        } else {
            inc_pos(buffer, self.pos.0, self.pos.1)
        };
        if step == -1 {
            return false;
        }
        if next.0 != self.pos.0 {
            self.line = line_chars(buffer, next.0);
            self.line_quotes = -1;
        }
        self.pos = next;
        true
    }

    fn process(&mut self) -> Option<(usize, usize)> {
        match self.line.get(self.pos.1).copied() {
            None => {
                if self.quotes == QuoteMode::Smart {
                    self.reset_at_nul();
                }
                None
            }
            Some('"') if self.quotes == QuoteMode::Smart => {
                self.toggle_quote();
                None
            }
            Some('\'') if self.quotes == QuoteMode::Smart => {
                self.skip_single_quote();
                None
            }
            Some(ch) => self.check_bracket(ch),
        }
    }

    fn reset_at_nul(&mut self) {
        let escaped = self.pos.1 > 0 && self.line.get(self.pos.1 - 1) == Some(&'\\');
        if self.pos.1 == 0 || !escaped {
            self.inquote = false;
            self.start_in_quotes = Some(false);
        }
    }

    fn toggle_quote(&mut self) {
        if self.line_quotes != 0 && backslash_run_before(&self.line, self.pos.1).is_multiple_of(2) {
            self.inquote = !self.inquote;
            self.start_in_quotes = Some(false);
        }
    }

    fn skip_single_quote(&mut self) -> bool {
        let col = self.pos.1;
        if self.backwards {
            if col > 1 {
                if self.line.get(col - 2) == Some(&'\'') {
                    self.pos.1 = col - 2;
                    return true;
                }
                if self.line.get(col - 2) == Some(&'\\')
                    && col > 2
                    && self.line.get(col - 3) == Some(&'\'')
                {
                    self.pos.1 = col - 3;
                    return true;
                }
            }
            return false;
        }
        if col + 1 >= self.line.len() {
            return false;
        }
        if self.line.get(col + 1) == Some(&'\\')
            && col + 2 < self.line.len()
            && self.line.get(col + 3) == Some(&'\'')
        {
            self.pos.1 = col + 3;
            return true;
        }
        if self.line.get(col + 2) == Some(&'\'') {
            self.pos.1 = col + 2;
            return true;
        }
        false
    }

    fn check_bracket(&mut self, ch: char) -> Option<(usize, usize)> {
        if self.inquote && self.start_in_quotes != Some(true) {
            return None;
        }
        if ch != self.initc && ch != self.findc {
            return None;
        }
        let escaped = backslash_run_before(&self.line, self.pos.1) % 2 == 1;
        if escaped != self.match_escaped {
            return None;
        }
        if ch == self.initc {
            self.depth += 1;
            return None;
        }
        if self.depth == 0 {
            return Some(self.pos);
        }
        self.depth -= 1;
        None
    }

    fn update_line_quotes(&mut self, buffer: &Buffer) {
        if self.line_quotes != -1 {
            return;
        }
        self.count_line_quotes();
        if self.line_quotes == 0 {
            self.inquote = false;
            self.apply_odd_line_rules(buffer);
        }
    }

    fn count_line_quotes(&mut self) {
        let mut count: i32 = -1;
        let mut index = 0;
        self.at_start = -1;
        while index < self.line.len() {
            if index == self.pos.1.saturating_add(usize::from(self.backwards)) {
                self.at_start = count & 1;
            }
            let ch = self.line[index];
            if ch == '"'
                && !(index > 0
                    && self.line[index - 1] == '\''
                    && self.line.get(index + 1) == Some(&'\''))
            {
                count += 1;
            }
            index += if ch == '\\' && index + 1 < self.line.len() {
                2
            } else {
                1
            };
        }
        self.line_quotes = count & 1;
    }

    fn apply_odd_line_rules(&mut self, buffer: &Buffer) {
        if self.line.last() == Some(&'\\') {
            self.line_quotes = 1;
            if self.start_in_quotes.is_none() {
                self.inquote = true;
                self.start_in_quotes = Some(true);
            } else if self.backwards {
                self.inquote = true;
            }
        }
        if self.pos.0 > 0 && line_ends_escaped(buffer, self.pos.0 - 1) {
            self.line_quotes = 1;
            if self.start_in_quotes.is_none() {
                self.inquote = self.at_start != 0;
                if self.inquote {
                    self.start_in_quotes = Some(true);
                }
            } else if !self.backwards {
                self.inquote = true;
            }
        }
    }
}

fn line_chars(buffer: &Buffer, row: usize) -> Vec<char> {
    buffer
        .line_cow(row)
        .map(|line| line.chars().collect())
        .unwrap_or_default()
}

fn line_ends_escaped(buffer: &Buffer, row: usize) -> bool {
    buffer
        .line_cow(row)
        .is_some_and(|line| line.ends_with('\\'))
}

fn backslash_run_before(line: &[char], col: usize) -> usize {
    let mut count = 0;
    let mut index = col;
    while index > 0 && line.get(index - 1) == Some(&'\\') {
        count += 1;
        index -= 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    fn find_close(
        buffer: &Buffer,
        pos: (usize, usize),
        open: char,
        close: char,
    ) -> Option<(usize, usize)> {
        find_close_forward(buffer, pos, MatchConfig::smart(open, close, false))
    }

    fn find_open(
        buffer: &Buffer,
        pos: (usize, usize),
        open: char,
        close: char,
    ) -> Option<(usize, usize)> {
        find_open_backward(buffer, pos, MatchConfig::smart(open, close, false))
    }

    #[test]
    fn forward_skips_bracket_inside_string() {
        let b = buf("x = f(\"a)b\");");
        assert_eq!(find_close(&b, (0, 5), '(', ')'), Some((0, 11)));
    }

    #[test]
    fn backward_stops_at_opening_quote() {
        let b = buf("x = f(\"a)b\");");
        assert_eq!(find_open(&b, (0, 8), '(', ')'), None);
    }

    #[test]
    fn forward_from_inside_string_matches_inside() {
        let b = buf("f(\"a(b)c\")");
        assert_eq!(find_close(&b, (0, 4), '(', ')'), Some((0, 6)));
    }

    #[test]
    fn forward_between_strings_matches_after() {
        let b = buf("a \"b\" (c) \"d\" e");
        assert_eq!(find_close(&b, (0, 6), '(', ')'), Some((0, 8)));
    }

    #[test]
    fn forward_on_odd_quote_line_ignores_quotes() {
        let b = buf("\"a\" (b) \"c");
        assert_eq!(find_close(&b, (0, 4), '(', ')'), Some((0, 6)));
    }

    #[test]
    fn forward_skips_single_quoted_chars() {
        let b = buf("a = ['(', ')']");
        assert_eq!(
            find_close_forward(&b, (0, 4), MatchConfig::smart('[', ']', false)),
            Some((0, 13))
        );
    }

    #[test]
    fn backward_skips_single_quoted_chars() {
        let b = buf("a = ['(', ')']");
        assert_eq!(
            find_open_backward(&b, (0, 13), MatchConfig::smart('[', ']', false)),
            Some((0, 4))
        );
    }

    #[test]
    fn forward_escaped_quote_keeps_string_open() {
        let b = buf("f(\"a\\\")b\")");
        assert_eq!(find_close(&b, (0, 1), '(', ')'), Some((0, 9)));
    }

    #[test]
    fn forward_matches_across_continued_string() {
        let b = buf("x = \"a\\\n(b) c\"");
        assert_eq!(find_close(&b, (1, 0), '(', ')'), Some((1, 2)));
    }

    #[test]
    fn forward_matches_across_unterminated_string() {
        let b = buf("x = \"a\n(b) c\"");
        assert_eq!(find_close(&b, (1, 0), '(', ')'), Some((1, 2)));
    }

    #[test]
    fn forward_ignores_quote_run_leak() {
        let b = buf("\"abc\ndef\"\n(foo)");
        assert_eq!(find_close(&b, (2, 0), '(', ')'), Some((2, 4)));
    }

    #[test]
    fn escaped_brackets_match_escaped() {
        let b = buf("\\(a\\)");
        let config = MatchConfig::smart('(', ')', true);
        assert_eq!(find_close_forward(&b, (0, 1), config), Some((0, 4)));
    }

    #[test]
    fn unescaped_bracket_skips_escaped_close() {
        let b = buf("(a\\)b)");
        assert_eq!(find_close(&b, (0, 0), '(', ')'), Some((0, 5)));
    }

    #[test]
    fn escaped_bracket_skips_unescaped_close() {
        let b = buf("\\(a)b\\)");
        let config = MatchConfig::smart('(', ')', true);
        assert_eq!(find_close_forward(&b, (0, 1), config), Some((0, 6)));
    }

    #[test]
    fn blind_forward_ignores_quotes() {
        let b = buf("f(\"a)b\")");
        assert_eq!(
            find_close_forward(&b, (0, 1), MatchConfig::blind('(', ')')),
            Some((0, 4))
        );
    }

    #[test]
    fn blind_backward_ignores_quotes() {
        let b = buf("f(\"a)b\")");
        assert_eq!(
            find_open_backward(&b, (0, 3), MatchConfig::blind('(', ')')),
            Some((0, 1))
        );
    }

    #[test]
    fn block_open_skips_unmatched_closes() {
        let b = buf("(a) (b)");
        assert_eq!(
            find_block_open(&b, (0, 3), MatchConfig::blind('(', ')')),
            Some((0, 4))
        );
    }

    #[test]
    fn starts_escaped_counts_backslashes() {
        let b = buf("a\\(b)");
        assert!(starts_escaped(&b, (0, 2)));
        assert!(!starts_escaped(&b, (0, 1)));
    }

    #[test]
    fn nested_string_scan_uses_depth() {
        let b = buf("f(\"a\" + g(x))");
        assert_eq!(find_close(&b, (0, 1), '(', ')'), Some((0, 12)));
    }
}
