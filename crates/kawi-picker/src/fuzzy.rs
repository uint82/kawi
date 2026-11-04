pub(crate) struct Query {
    needle: String,
    case_sensitive: bool,
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
        score(&self.needle, haystack)
    }
}

pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let query = Query::new(query);
    if query.is_empty() {
        return Some(0);
    }
    query.score(candidate, &candidate.to_lowercase())
}

fn score(needle: &str, haystack: &str) -> Option<i32> {
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
}
