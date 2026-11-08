use std::ops::Range;

use kawi_buffer::{diff_ropes, RopeChange};
use kawi_completion::Snippet;
use ropey::Rope;

pub(crate) struct SnippetSession {
    stops: Vec<Range<usize>>,
    final_stop: usize,
    active: usize,
    snapshot: Rope,
}

pub(crate) enum SnippetStep {
    Select(Range<usize>),
    Finished(usize),
}

impl SnippetSession {
    pub(crate) fn new(snippet: &Snippet, base: usize, snapshot: Rope) -> Option<Self> {
        if snippet.stops().is_empty() {
            return None;
        }
        let limit = snapshot.len_chars();
        let stops = snippet
            .stops()
            .iter()
            .map(|stop| {
                clamp_range(
                    base.saturating_add(stop.range().start),
                    base.saturating_add(stop.range().end),
                    limit,
                )
            })
            .collect();
        Some(Self {
            stops,
            final_stop: base.saturating_add(snippet.final_stop()).min(limit),
            active: 0,
            snapshot,
        })
    }

    pub(crate) fn active_range(&self) -> Range<usize> {
        self.stops[self.active].clone()
    }

    pub(crate) fn next(&mut self) -> SnippetStep {
        if let Some(range) = self.stops.get(self.active + 1) {
            self.active += 1;
            return SnippetStep::Select(range.clone());
        }
        SnippetStep::Finished(self.final_stop)
    }

    pub(crate) fn previous(&mut self) -> Option<Range<usize>> {
        let next = self.active.checked_sub(1)?;
        self.active = next;
        Some(self.stops[next].clone())
    }

    pub(crate) fn update(&mut self, current: &Rope) {
        let Some(change) = diff_ropes(&self.snapshot, current) else {
            return;
        };
        for stop in &mut self.stops {
            *stop = remap_range(stop, &change);
        }
        self.final_stop = remap_position(self.final_stop, &change);
        self.snapshot = current.clone();
    }
}

fn clamp_range(start: usize, end: usize, limit: usize) -> Range<usize> {
    let start = start.min(limit);
    let end = end.min(limit);
    start..end.max(start)
}

fn remap_range(range: &Range<usize>, change: &RopeChange) -> Range<usize> {
    if range.end <= change.start {
        return range.clone();
    }
    if range.start >= change.old_end {
        return shift(range.start, change)..shift(range.end, change);
    }
    let start = range.start.min(change.start);
    let end = if range.end >= change.old_end {
        shift(range.end, change)
    } else {
        change.new_end
    };
    start..end.max(start)
}

fn remap_position(position: usize, change: &RopeChange) -> usize {
    if position < change.start {
        return position;
    }
    if position >= change.old_end {
        return shift(position, change);
    }
    change.new_end
}

fn shift(value: usize, change: &RopeChange) -> usize {
    value.saturating_add_signed(change.new_end as isize - change.old_end as isize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(source: &str) -> Snippet {
        Snippet::parse(source)
    }

    fn session(source: &str, base: usize, text: &str) -> SnippetSession {
        SnippetSession::new(&snippet(source), base, Rope::from_str(text)).expect("session")
    }

    #[test]
    fn empty_stop_lists_do_not_create_sessions() {
        assert!(SnippetSession::new(&snippet("plain"), 0, Rope::new()).is_none());
    }

    #[test]
    fn stops_are_offset_by_the_insertion_point() {
        let session = session("$1 ${2:two}", 5, "abcdefghij");
        assert_eq!(session.active_range(), 5..5);
    }

    #[test]
    fn next_walks_to_the_final_position() {
        let mut session = session("$1 $2", 0, " ");
        assert!(matches!(session.next(), SnippetStep::Select(range) if range == (1..1)));
        assert!(matches!(session.next(), SnippetStep::Finished(position) if position == 1));
    }

    #[test]
    fn previous_walks_back_without_leaving_the_first_stop() {
        let mut session = session("$1 $2", 0, "$1 $2");
        session.next();
        assert_eq!(session.previous(), Some(0..0));
        assert_eq!(session.previous(), None);
    }

    #[test]
    fn edits_before_a_stop_shift_it() {
        let mut session = session("$1", 4, "abcd");
        session.update(&Rope::from_str("abXcd"));
        assert_eq!(session.active_range(), 5..5);
    }

    #[test]
    fn replacing_a_stop_selects_the_replacement() {
        let mut session = session("${1:name}", 0, "name");
        session.update(&Rope::from_str("value"));
        assert_eq!(session.active_range(), 0..5);
    }

    #[test]
    fn edits_track_the_final_position() {
        let mut session = session("$1$0", 0, "");
        session.update(&Rope::from_str("abc"));
        assert!(matches!(session.next(), SnippetStep::Finished(position) if position == 3));
    }

    #[test]
    fn deletions_collapse_a_stop() {
        let mut session = session("${1:name}", 3, "name");
        session.update(&Rope::from_str(""));
        assert_eq!(session.active_range(), 0..0);
    }
}
