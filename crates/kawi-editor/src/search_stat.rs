use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use ropey::RopeSlice;

use crate::matcher::{self, Query};
use crate::search::{position_byte, slice_text};

pub const MATCH_STAT_MAX: usize = 999;
pub const MATCH_STAT_TIMEOUT: Duration = Duration::from_millis(40);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchStatKind {
    Complete,
    Capped,
    TimedOut,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MatchStat {
    pub current: usize,
    pub total: usize,
    pub kind: MatchStatKind,
}

pub fn search_stat(
    buffer: &Buffer,
    query: &Query,
    row: usize,
    col: usize,
    maxcount: usize,
    timeout: Duration,
) -> MatchStat {
    if query.is_empty() {
        return MatchStat {
            current: 0,
            total: 0,
            kind: MatchStatKind::Complete,
        };
    }
    let slice = buffer.rope_slice();
    let cursor_byte = position_byte(&slice, buffer, row, col);
    let mut tally = StatTally::new(cursor_byte, maxcount, Instant::now() + timeout);
    if query.needs_folded_scan() {
        tally_folded_lines(buffer, &slice, query, &mut tally);
    } else {
        let mut visit = |hit| tally.visit(hit);
        matcher::scan_in_range(&slice, query, 0, slice.len_bytes(), &mut visit);
    }
    tally.stat
}

struct StatTally {
    cursor_byte: usize,
    maxcount: usize,
    deadline: Instant,
    stat: MatchStat,
}

impl StatTally {
    fn new(cursor_byte: usize, maxcount: usize, deadline: Instant) -> Self {
        Self {
            cursor_byte,
            maxcount,
            deadline,
            stat: MatchStat {
                current: 0,
                total: 0,
                kind: MatchStatKind::Complete,
            },
        }
    }

    fn visit(&mut self, hit: usize) -> bool {
        if Instant::now() >= self.deadline {
            self.stat.kind = MatchStatKind::TimedOut;
            return false;
        }
        self.stat.total = self.stat.total.saturating_add(1);
        if hit <= self.cursor_byte {
            self.stat.current = self.stat.current.saturating_add(1);
        }
        if self.stat.total > self.maxcount {
            self.stat.kind = MatchStatKind::Capped;
            return false;
        }
        true
    }
}

fn tally_folded_lines(buffer: &Buffer, slice: &RopeSlice, query: &Query, tally: &mut StatTally) {
    for (row, line) in buffer.line_slices().enumerate() {
        let text = slice_text(&line);
        let mut bound = 0;
        while let Some(col) = query.first_at_or_after(&text, bound) {
            let byte = position_byte(slice, buffer, row, col);
            if !tally.visit(byte) {
                return;
            }
            bound = col.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folded(pattern: &str) -> Query {
        Query::new(pattern, false, false)
    }

    fn lines_of(text: &str) -> Buffer {
        Buffer::from_string(text)
    }

    fn stat_at(buffer: &Buffer, query: &Query, row: usize, col: usize) -> MatchStat {
        search_stat(
            buffer,
            query,
            row,
            col,
            usize::MAX,
            std::time::Duration::from_secs(10),
        )
    }

    #[test]
    fn stat_counts_all_matches() {
        let lines = lines_of("ar ar\nar");
        assert_eq!(stat_at(&lines, &folded("ar"), 0, 0).total, 3);
        assert_eq!(stat_at(&lines, &folded(""), 0, 0).total, 0);
        assert_eq!(stat_at(&lines, &folded("zzz"), 0, 0).total, 0);
    }

    #[test]
    fn stat_ranks_match_at_cursor() {
        let lines = lines_of("ar ar\nar");
        assert_eq!(stat_at(&lines, &folded("ar"), 0, 0).current, 1);
        assert_eq!(stat_at(&lines, &folded("ar"), 0, 3).current, 2);
        assert_eq!(stat_at(&lines, &folded("ar"), 1, 0).current, 3);
    }

    #[test]
    fn stat_counts_matches_up_to_cursor() {
        let lines = lines_of("ar ar");
        assert_eq!(stat_at(&lines, &folded("ar"), 0, 5).current, 2);
        assert_eq!(stat_at(&lines, &folded(""), 0, 0).current, 0);
    }

    #[test]
    fn stat_counts_unicode_pattern() {
        let lines = lines_of("CAFÉ café");
        assert_eq!(stat_at(&lines, &folded("café"), 0, 0).total, 2);
        assert_eq!(stat_at(&lines, &folded("café"), 0, 6).current, 2);
    }

    #[test]
    fn stat_caps_total_at_maxcount() {
        let lines = lines_of("ar ar ar ar ar");
        let stat = search_stat(
            &lines,
            &folded("ar"),
            0,
            6,
            2,
            std::time::Duration::from_secs(10),
        );
        assert_eq!(stat.current, 3);
        assert_eq!(stat.total, 3);
        assert_eq!(stat.kind, MatchStatKind::Capped);
    }

    #[test]
    fn stat_reports_timeout() {
        let lines = lines_of("ar ar");
        let stat = search_stat(&lines, &folded("ar"), 0, 0, 999, std::time::Duration::ZERO);
        assert_eq!(stat.kind, MatchStatKind::TimedOut);
    }

    #[test]
    fn stat_counts_whole_words() {
        let lines = lines_of("foo foobar foo");
        let query = Query::new("foo", true, true);
        let stat = stat_at(&lines, &query, 0, 0);
        assert_eq!(stat.total, 2);
        assert_eq!(stat.current, 1);
    }
}
