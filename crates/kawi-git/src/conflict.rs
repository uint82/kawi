use std::ops::Range;

const START_MARKER: &str = "<<<<<<<";
const BASE_MARKER: &str = "|||||||";
const SEPARATOR: &str = "=======";
const END_MARKER: &str = ">>>>>>>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    Ours,
    Theirs,
    Both,
}

impl ConflictChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ours => "ours",
            Self::Theirs => "theirs",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictBlock {
    pub start: usize,
    pub base: Option<usize>,
    pub separator: usize,
    pub end: usize,
}

impl ConflictBlock {
    pub fn start_line(&self) -> usize {
        self.start.saturating_add(1)
    }

    pub fn end_line(&self) -> usize {
        self.end.saturating_add(1)
    }

    pub fn contains_line(&self, line: usize) -> bool {
        self.start_line() <= line && line <= self.end_line()
    }

    pub fn ours(&self) -> Range<usize> {
        let end = self.base.unwrap_or(self.separator);
        self.start.saturating_add(1)..end
    }

    pub fn base(&self) -> Option<Range<usize>> {
        self.base.map(|base| base.saturating_add(1)..self.separator)
    }

    pub fn theirs(&self) -> Range<usize> {
        self.separator.saturating_add(1)..self.end
    }
}

#[derive(Default)]
struct ParseState {
    start: Option<usize>,
    base: Option<usize>,
    separator: Option<usize>,
}

impl ParseState {
    fn observe(&mut self, index: usize, line: &str) -> Option<ConflictBlock> {
        if is_start(line) {
            self.start = Some(index);
            self.base = None;
            self.separator = None;
            return None;
        }
        self.start?;
        if self.base.is_none() && self.separator.is_none() && is_base(line) {
            self.base = Some(index);
            return None;
        }
        if self.separator.is_none() && line == SEPARATOR {
            self.separator = Some(index);
            return None;
        }
        if !is_end(line) {
            return None;
        }
        let block = self.finish(index);
        self.start = None;
        self.base = None;
        self.separator = None;
        block
    }

    fn finish(&self, end: usize) -> Option<ConflictBlock> {
        Some(ConflictBlock {
            start: self.start?,
            base: self.base,
            separator: self.separator?,
            end,
        })
    }
}

pub fn parse<T>(lines: impl IntoIterator<Item = T>) -> Vec<ConflictBlock>
where
    T: AsRef<str>,
{
    let mut state = ParseState::default();
    lines
        .into_iter()
        .enumerate()
        .filter_map(|(index, line)| {
            state.observe(index, line.as_ref().trim_end_matches(['\r', '\n']))
        })
        .collect()
}

pub fn has_markers<'a>(lines: impl IntoIterator<Item = &'a str>) -> bool {
    lines
        .into_iter()
        .any(|line| is_start(line) || is_base(line) || is_end(line))
}

pub fn at(blocks: &[ConflictBlock], line: usize) -> Option<usize> {
    blocks.iter().position(|block| block.contains_line(line))
}

pub fn next(blocks: &[ConflictBlock], line: usize) -> Option<usize> {
    if blocks.is_empty() {
        return None;
    }
    Some(
        blocks
            .iter()
            .position(|block| block.start_line() > line)
            .unwrap_or(0),
    )
}

pub fn prev(blocks: &[ConflictBlock], line: usize) -> Option<usize> {
    if blocks.is_empty() {
        return None;
    }
    Some(
        blocks
            .iter()
            .rposition(|block| block.end_line() < line)
            .unwrap_or(blocks.len().saturating_sub(1)),
    )
}

pub fn replacement<'a>(
    lines: &'a [String],
    block: &ConflictBlock,
    choice: ConflictChoice,
) -> Vec<&'a str> {
    let ours = line_slice(lines, block.ours());
    let theirs = line_slice(lines, block.theirs());
    match choice {
        ConflictChoice::Ours => ours.iter().map(String::as_str).collect(),
        ConflictChoice::Theirs => theirs.iter().map(String::as_str).collect(),
        ConflictChoice::Both => ours.iter().chain(theirs).map(String::as_str).collect(),
    }
}

fn line_slice(lines: &[String], range: Range<usize>) -> &[String] {
    lines.get(range).unwrap_or(&[])
}

fn is_start(line: &str) -> bool {
    line.starts_with(START_MARKER)
}

fn is_base(line: &str) -> bool {
    line.starts_with(BASE_MARKER)
}

fn is_end(line: &str) -> bool {
    line.starts_with(END_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn parses_two_way_conflicts() {
        let input = lines(&[
            "before",
            "<<<<<<< HEAD",
            "ours",
            "=======",
            "theirs",
            ">>>>>>> topic",
            "after",
        ]);
        let blocks = parse(input.iter().map(String::as_str));
        assert_eq!(
            blocks,
            vec![ConflictBlock {
                start: 1,
                base: None,
                separator: 3,
                end: 5,
            }]
        );
        assert_eq!(blocks[0].ours(), 2..3);
        assert_eq!(blocks[0].theirs(), 4..5);
        assert_eq!(blocks[0].base(), None);
    }

    #[test]
    fn parses_diff3_conflicts() {
        let input = lines(&[
            "<<<<<<< HEAD",
            "ours",
            "||||||| base",
            "base",
            "=======",
            "theirs",
            ">>>>>>> topic",
        ]);
        let blocks = parse(input.iter().map(String::as_str));
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].base, Some(2));
        assert_eq!(blocks[0].base(), Some(3..4));
        assert_eq!(blocks[0].ours(), 1..2);
        assert_eq!(blocks[0].theirs(), 5..6);
    }

    #[test]
    fn ignores_separators_outside_conflicts() {
        let input = lines(&["Title", "=======", "Body"]);
        assert!(parse(input.iter().map(String::as_str)).is_empty());
    }

    #[test]
    fn parses_lines_with_endings() {
        let input = [
            "before\n",
            "<<<<<<< HEAD\n",
            "ours\n",
            "=======\n",
            "theirs\n",
            ">>>>>>> topic\n",
        ];
        let blocks = parse(input);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start, 1);
        assert_eq!(blocks[0].separator, 3);
        assert_eq!(blocks[0].end, 5);
    }

    #[test]
    fn incomplete_blocks_are_not_parsed_but_are_markers() {
        let input = lines(&["<<<<<<< HEAD", "ours", "=======", "theirs"]);
        assert!(parse(input.iter().map(String::as_str)).is_empty());
        assert!(has_markers(input.iter().map(String::as_str)));
    }

    #[test]
    fn replacement_selects_each_side() {
        let input = lines(&["<<<<<<< HEAD", "ours", "=======", "theirs", ">>>>>>> topic"]);
        let blocks = parse(input.iter().map(String::as_str));
        let block = &blocks[0];
        assert_eq!(
            replacement(&input, block, ConflictChoice::Ours),
            vec!["ours"]
        );
        assert_eq!(
            replacement(&input, block, ConflictChoice::Theirs),
            vec!["theirs"]
        );
        assert_eq!(
            replacement(&input, block, ConflictChoice::Both),
            vec!["ours", "theirs"]
        );
    }

    #[test]
    fn navigation_wraps_between_blocks() {
        let input = lines(&[
            "<<<<<<< A",
            "a",
            "=======",
            "b",
            ">>>>>>> A",
            "<<<<<<< B",
            "c",
            "=======",
            "d",
            ">>>>>>> B",
        ]);
        let blocks = parse(input.iter().map(String::as_str));
        assert_eq!(next(&blocks, 1), Some(1));
        assert_eq!(next(&blocks, 6), Some(0));
        assert_eq!(prev(&blocks, 6), Some(0));
        assert_eq!(prev(&blocks, 1), Some(1));
        assert_eq!(at(&blocks, 3), Some(0));
        assert_eq!(at(&blocks, 8), Some(1));
        assert_eq!(at(&blocks, 11), None);
    }
}
