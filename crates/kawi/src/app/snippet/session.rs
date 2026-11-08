use std::ops::Range;

use kawi_buffer::{diff_ropes, RopeChange, TextEdit};
use kawi_completion::Snippet;
use ropey::Rope;

pub(crate) struct SnippetSession {
    stops: Vec<StopGroup>,
    final_stop: usize,
    active: usize,
    snapshot: Rope,
}

struct StopGroup {
    index: usize,
    choices: Vec<String>,
    ranges: Vec<Range<usize>>,
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
        let mut stops: Vec<StopGroup> = Vec::new();
        for stop in snippet.stops() {
            let range = clamp_range(
                base.saturating_add(stop.range().start),
                base.saturating_add(stop.range().end),
                limit,
            );
            match stops.last_mut() {
                Some(group) if group.index == stop.index() => group.ranges.push(range),
                _ => stops.push(StopGroup {
                    index: stop.index(),
                    choices: stop.choices().to_vec(),
                    ranges: vec![range],
                }),
            }
        }
        Some(Self {
            stops,
            final_stop: base.saturating_add(snippet.final_stop()).min(limit),
            active: 0,
            snapshot,
        })
    }

    pub(crate) fn active_range(&self) -> Range<usize> {
        self.stops[self.active].ranges[0].clone()
    }

    pub(crate) fn active_choices(&self) -> Option<&[String]> {
        let choices = &self.stops[self.active].choices;
        (!choices.is_empty()).then_some(choices)
    }

    pub(crate) fn next(&mut self) -> SnippetStep {
        if let Some(group) = self.stops.get(self.active + 1) {
            self.active += 1;
            return SnippetStep::Select(group.ranges[0].clone());
        }
        SnippetStep::Finished(self.final_stop)
    }

    pub(crate) fn previous(&mut self) -> Option<Range<usize>> {
        let next = self.active.checked_sub(1)?;
        self.active = next;
        Some(self.stops[next].ranges[0].clone())
    }

    pub(crate) fn update(&mut self, current: &Rope) {
        let Some(change) = diff_ropes(&self.snapshot, current) else {
            return;
        };
        for group in &mut self.stops {
            for range in &mut group.ranges {
                *range = remap_range(range, &change);
            }
        }
        self.final_stop = remap_position(self.final_stop, &change);
        self.snapshot = current.clone();
    }

    pub(crate) fn mirror_edits(&self, text: &Rope) -> Vec<TextEdit> {
        let Some(group) = self.stops.get(self.active) else {
            return Vec::new();
        };
        let Some(primary) = group.ranges.first() else {
            return Vec::new();
        };
        let value = slice(text, primary);
        group
            .ranges
            .iter()
            .skip(1)
            .filter(|range| slice(text, range) != value)
            .map(|range| TextEdit {
                start: range.start,
                end: range.end,
                text: value.to_string(),
            })
            .collect()
    }
}

fn slice(text: &Rope, range: &Range<usize>) -> String {
    text.slice(range.start..range.end).to_string()
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

    #[test]
    fn duplicate_stops_form_one_navigation_group() {
        let mut session = session("$1 $1", 0, " ");
        assert_eq!(session.active_range(), 0..0);
        assert!(matches!(session.next(), SnippetStep::Finished(position) if position == 1));
    }

    #[test]
    fn mirror_edits_copy_the_active_stop_text() {
        let session = session("${1:one} ${1:two}", 0, "one two");
        let edits = session.mirror_edits(&Rope::from_str("one two"));
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start, 4);
        assert_eq!(edits[0].end, 7);
        assert_eq!(edits[0].text, "one");
    }

    #[test]
    fn mirror_edits_track_edits_to_the_active_stop() {
        let mut session = session("${1:one} ${1:two}", 0, "one two");
        session.update(&Rope::from_str("ONE two"));
        let edits = session.mirror_edits(&Rope::from_str("ONE two"));
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start, 4);
        assert_eq!(edits[0].end, 7);
        assert_eq!(edits[0].text, "ONE");
    }
}
