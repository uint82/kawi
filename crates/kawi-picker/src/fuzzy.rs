use std::ops::Range;

pub(crate) struct Query {
    needle: String,
    case_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    score: i32,
    ranges: Vec<Range<usize>>,
}

impl FuzzyMatch {
    pub fn score(&self) -> i32 {
        self.score
    }

    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}

pub struct FuzzyMatcher {
    query: Query,
}

impl FuzzyMatcher {
    pub fn new(query: &str) -> Self {
        Self {
            query: Query::new(query),
        }
    }

    pub fn matches(&self, candidate: &str) -> Option<FuzzyMatch> {
        if self.query.is_empty() {
            return Some(FuzzyMatch {
                score: 0,
                ranges: Vec::new(),
            });
        }
        let (haystack, offsets) = fold_with_offsets(candidate, self.query.case_sensitive);
        let mut positions = Vec::new();
        let score = score(&self.query.needle, &haystack, Some(&mut positions))?;
        let ranges = offsets
            .map(|offsets| byte_ranges(candidate, &offsets, &positions))
            .unwrap_or_default();
        Some(FuzzyMatch { score, ranges })
    }
}

impl Query {
    pub(crate) fn new(query: &str) -> Self {
        let case_sensitive = query.chars().any(char::is_uppercase);
        Self {
            needle: fold(query, case_sensitive),
            case_sensitive,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.needle.is_empty()
    }

    pub(crate) fn score(&self, candidate: &str, folded: &str) -> Option<i32> {
        if self.is_empty() {
            return Some(0);
        }
        let haystack = if self.case_sensitive {
            candidate
        } else {
            folded
        };
        score(&self.needle, haystack, None)
    }
}

pub fn fuzzy_match(query: &str, candidate: &str) -> Option<FuzzyMatch> {
    FuzzyMatcher::new(query).matches(candidate)
}

pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    fuzzy_match(query, candidate).map(|matched| matched.score)
}

fn score(needle: &str, haystack: &str, mut positions: Option<&mut Vec<usize>>) -> Option<i32> {
    let haystack_chars: Vec<char> = haystack.chars().collect();
    let basename_start = haystack.rfind('/').map_or(0, |index| index + 1);
    let mut score = 0;
    let mut position = 0;
    let mut previous = None;
    for (index, needle_char) in needle.chars().enumerate() {
        let found = haystack_chars[position..]
            .iter()
            .position(|haystack_char| *haystack_char == needle_char)?;
        let found = position + found;
        let mut points = 10;
        if previous.is_some_and(|last| found == last + 1) {
            points += 15;
        }
        if found == 0 || is_boundary(haystack_chars[found - 1]) {
            points += 20;
        }
        if found >= basename_start {
            points += 5;
        }
        if index == 0 && found >= basename_start {
            points += 15;
        }
        if let Some(positions) = positions.as_deref_mut() {
            positions.push(found);
        }
        score += points;
        previous = Some(found);
        position = found + 1;
    }
    if haystack.starts_with(needle) {
        score += 30;
    }
    if haystack[basename_start..].starts_with(needle) {
        score += 20;
    }
    Some(score - (haystack_chars.len() / 10) as i32)
}

fn fold_with_offsets(text: &str, case_sensitive: bool) -> (String, Option<Vec<usize>>) {
    if case_sensitive {
        let offsets = text.char_indices().map(|(index, _)| index).collect();
        return (text.to_string(), Some(offsets));
    }
    let folded = text.to_lowercase();
    let mut offsets = Vec::new();
    for (index, character) in text.char_indices() {
        for _ in character.to_lowercase() {
            offsets.push(index);
        }
    }
    if offsets.len() == folded.chars().count() {
        (folded, Some(offsets))
    } else {
        (folded, None)
    }
}

fn byte_ranges(candidate: &str, offsets: &[usize], positions: &[usize]) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::with_capacity(positions.len());
    for position in positions {
        let Some(&start) = offsets.get(*position) else {
            continue;
        };
        let Some(character) = candidate[start..].chars().next() else {
            continue;
        };
        let end = start.saturating_add(character.len_utf8());
        match ranges.last_mut() {
            Some(range) if range.end == start => range.end = end,
            Some(range) if range.start <= start && start < range.end => {}
            _ => ranges.push(start..end),
        }
    }
    ranges
}

fn fold(text: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        text.to_string()
    } else {
        text.to_lowercase()
    }
}

fn is_boundary(character: char) -> bool {
    matches!(character, '/' | '\\' | '_' | '-' | '.' | ' ' | '\t')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_prefix_queries_rank_above_scattered_matches() {
        let exact = fuzzy_score("main", "src/main.rs").expect("exact");
        let scattered = fuzzy_score("main", "src/domain/input.rs").expect("scattered");
        assert!(exact > scattered);
    }

    #[test]
    fn basename_matches_rank_above_path_matches() {
        let basename = fuzzy_score("lib", "crates/kawi/lib.rs").expect("basename");
        let path = fuzzy_score("lib", "lib/crates/kawi.rs").expect("path");
        assert!(basename > path);
    }

    #[test]
    fn smart_case_ignores_case_for_lowercase_queries() {
        assert!(fuzzy_score("readme", "README.md").is_some());
        assert!(fuzzy_score("Readme", "README.md").is_none());
    }

    #[test]
    fn non_subsequence_does_not_match() {
        assert_eq!(fuzzy_score("xyz", "src/main.rs"), None);
    }

    #[test]
    fn empty_query_matches_everything() {
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn match_ranges_point_at_the_matched_characters() {
        let matched = fuzzy_match("mn", "main").expect("match");
        assert_eq!(matched.ranges(), [0..1, 3..4]);
    }

    #[test]
    fn consecutive_characters_merge_into_one_range() {
        let matched = fuzzy_match("ma", "main").expect("match");
        assert_eq!(matched.ranges(), [0..2]);
    }

    #[test]
    fn ranges_keep_original_byte_offsets_for_wide_characters() {
        let matched = fuzzy_match("ol", "école").expect("match");
        assert_eq!(matched.ranges(), [3..5]);
    }

    #[test]
    fn case_insensitive_matches_keep_original_offsets() {
        let matched = fuzzy_match("é", "École").expect("match");
        assert_eq!(matched.ranges(), [0..2]);
    }

    #[test]
    fn empty_queries_have_no_ranges() {
        let matched = fuzzy_match("", "anything").expect("match");
        assert!(matched.ranges().is_empty());
    }

    #[test]
    fn scorer_and_match_report_the_same_score() {
        for (query, candidate) in [("main", "src/main.rs"), ("mn", "main"), ("", "main")] {
            assert_eq!(
                fuzzy_score(query, candidate),
                fuzzy_match(query, candidate).map(|matched| matched.score())
            );
        }
    }
}
