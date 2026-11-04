use super::Buffer;
use crate::unicode;

impl Buffer {
    pub fn line_grapheme_count(&self, row: usize) -> usize {
        self.line_cow(row)
            .map_or(0, |line| unicode::graphemes(&line).count())
    }

    pub fn last_grapheme_col(&self, row: usize) -> usize {
        self.line_cow(row)
            .map_or(0, |line| unicode::last_boundary(&line))
    }

    pub fn prev_grapheme_col(&self, row: usize, col: usize) -> usize {
        self.line_cow(row)
            .map_or(0, |line| unicode::prev_boundary(&line, col))
    }

    pub fn next_grapheme_col(&self, row: usize, col: usize) -> usize {
        self.line_cow(row)
            .map_or(0, |line| unicode::next_boundary(&line, col))
    }

    pub fn snap_col(&self, row: usize, col: usize) -> usize {
        self.line_cow(row)
            .map_or(0, |line| unicode::cluster_start(&line, col))
    }

    pub fn grapheme_end_after(&self, row: usize, col: usize, count: usize) -> usize {
        let mut end = self.snap_col(row, col);
        for _ in 0..count {
            let next = self.next_grapheme_col(row, end);
            if next == end {
                break;
            }
            end = next;
        }
        end
    }

    pub fn grapheme_count_from(&self, row: usize, col: usize) -> usize {
        let mut col = self.snap_col(row, col);
        let mut count = 0;
        while col < self.line_len(row) {
            let next = self.next_grapheme_col(row, col);
            if next == col {
                break;
            }
            col = next;
            count += 1;
        }
        count
    }

    pub fn grapheme_text(&self, row: usize, col: usize) -> Option<String> {
        let line = self.line_cow(row)?;
        unicode::grapheme_at(&line, col).map(|grapheme| grapheme.text().to_string())
    }

    pub fn grapheme_span(&self, row: usize, col: usize) -> Option<(usize, usize)> {
        let line = self.line_cow(row)?;
        let grapheme = unicode::grapheme_at(&line, col)?;
        Some((grapheme.start(), grapheme.end()))
    }

    pub fn screen_span(&self, row: usize, col: usize, tabstop: usize) -> (usize, usize) {
        let start_col = self.snap_col(row, col);
        let end_col = self
            .line_cow(row)
            .map_or(start_col, |line| unicode::cluster_end(&line, col));
        (
            self.screen_col(row, start_col, tabstop),
            self.screen_col(row, end_col, tabstop),
        )
    }
}
