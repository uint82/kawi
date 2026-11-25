use std::fmt;

use super::COMMAND_NAMES;

const MAX_SUGGESTIONS: usize = 3;
const SHORT_INPUT: usize = 2;
const MEDIUM_INPUT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    ArgumentRequired,
    InvalidArgument,
    NotAnEditorCommand,
    TrailingCharacters,
}

impl DiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArgumentRequired => "E471",
            Self::InvalidArgument => "E474",
            Self::NotAnEditorCommand => "E492",
            Self::TrailingCharacters => "E488",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseFailure {
    UnknownCommand { name: String },
    MissingArgument { command: String },
    InvalidArgument { argument: String },
}

impl ParseFailure {
    pub const fn code(&self) -> DiagnosticCode {
        match self {
            Self::UnknownCommand { .. } => DiagnosticCode::NotAnEditorCommand,
            Self::MissingArgument { .. } => DiagnosticCode::ArgumentRequired,
            Self::InvalidArgument { .. } => DiagnosticCode::InvalidArgument,
        }
    }
}

fn is_known_command(name: &str) -> bool {
    COMMAND_NAMES.contains(&name)
        || name
            .strip_suffix('!')
            .is_some_and(|base| COMMAND_NAMES.contains(&base))
}

pub fn classify_parse_failure(name: &str, rest: &str) -> Option<ParseFailure> {
    if name.is_empty() {
        return None;
    }
    if !is_known_command(name) {
        return Some(ParseFailure::UnknownCommand {
            name: name.to_string(),
        });
    }
    if rest.is_empty() {
        Some(ParseFailure::MissingArgument {
            command: name.to_string(),
        })
    } else {
        Some(ParseFailure::InvalidArgument {
            argument: rest.to_string(),
        })
    }
}

pub fn suggest_commands(input: &str, candidates: &[&str]) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return Vec::new();
    }
    let normalized = input.to_ascii_lowercase();
    let limit = max_distance(normalized.chars().count());
    if limit == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(usize, &str)> = candidates
        .iter()
        .filter_map(|candidate| score(input, &normalized, candidate))
        .filter(|(distance, _)| *distance <= limit)
        .collect();
    scored.sort();
    scored
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, candidate)| candidate.to_string())
        .collect()
}

fn max_distance(length: usize) -> usize {
    if length <= SHORT_INPUT {
        0
    } else if length <= MEDIUM_INPUT {
        2
    } else {
        3
    }
}

fn score<'a>(original: &str, normalized: &str, candidate: &'a str) -> Option<(usize, &'a str)> {
    if candidate == original {
        return None;
    }
    let candidate_normalized = candidate.to_ascii_lowercase();
    let distance = edit_distance(normalized, &candidate_normalized);
    let first_matches = normalized.chars().next() == candidate_normalized.chars().next();
    (first_matches || distance <= 1).then_some((distance, candidate))
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len().saturating_add(1)];
    for (row, left_char) in left.chars().enumerate() {
        current[0] = row.saturating_add(1);
        for (column, right_char) in right.iter().enumerate() {
            let replace = previous[column].saturating_add(usize::from(left_char != *right_char));
            let insert = current[column].saturating_add(1);
            let delete = previous[column.saturating_add(1)].saturating_add(1);
            current[column.saturating_add(1)] = replace.min(insert).min(delete);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_unknown_command() {
        assert_eq!(
            classify_parse_failure("hlep", ""),
            Some(ParseFailure::UnknownCommand {
                name: "hlep".to_string()
            })
        );
    }

    #[test]
    fn classifies_missing_argument() {
        assert_eq!(
            classify_parse_failure("b", ""),
            Some(ParseFailure::MissingArgument {
                command: "b".to_string()
            })
        );
    }

    #[test]
    fn classifies_invalid_argument() {
        assert_eq!(
            classify_parse_failure("bd", "a b"),
            Some(ParseFailure::InvalidArgument {
                argument: "a b".to_string()
            })
        );
    }

    #[test]
    fn force_suffixes_of_known_commands_are_known() {
        assert_eq!(
            classify_parse_failure("e!", "a b"),
            Some(ParseFailure::InvalidArgument {
                argument: "a b".to_string()
            })
        );
        assert_eq!(
            classify_parse_failure("bd!", "x"),
            Some(ParseFailure::InvalidArgument {
                argument: "x".to_string()
            })
        );
    }

    #[test]
    fn ignores_empty_input() {
        assert_eq!(classify_parse_failure("", ""), None);
    }

    #[test]
    fn maps_failures_to_codes() {
        assert_eq!(
            ParseFailure::UnknownCommand {
                name: "nope".to_string()
            }
            .code(),
            DiagnosticCode::NotAnEditorCommand
        );
        assert_eq!(
            ParseFailure::MissingArgument {
                command: "b".to_string()
            }
            .code(),
            DiagnosticCode::ArgumentRequired
        );
        assert_eq!(
            ParseFailure::InvalidArgument {
                argument: "x".to_string()
            }
            .code(),
            DiagnosticCode::InvalidArgument
        );
    }

    #[test]
    fn suggests_a_near_miss() {
        assert_eq!(suggest_commands("hlep", &["help", "health"]), ["help"]);
    }

    #[test]
    fn suggests_case_insensitive_match() {
        assert_eq!(suggest_commands("HELP", &["help"]), ["help"]);
    }

    #[test]
    fn suggests_an_abbreviation() {
        assert_eq!(suggest_commands("spl", &["split", "sp"]), ["sp", "split"]);
    }

    #[test]
    fn ignores_short_unknown_names() {
        assert!(suggest_commands("x", &["q", "w"]).is_empty());
    }

    #[test]
    fn ignores_distant_names() {
        assert!(suggest_commands("zzzzzz", &["help"]).is_empty());
    }

    #[test]
    fn ignores_unrelated_names_with_a_shared_suffix() {
        assert!(suggest_commands("hlep", &["help", "lsp"]).contains(&"help".to_string()));
        assert!(!suggest_commands("hlep", &["help", "lsp"]).contains(&"lsp".to_string()));
    }

    #[test]
    fn caps_suggestions_and_orders_by_distance() {
        let suggestions = suggest_commands("helt", &["health", "help", "held", "hello"]);
        assert_eq!(suggestions.len(), MAX_SUGGESTIONS);
        assert_eq!(suggestions[0], "held");
        assert!(suggestions.contains(&"help".to_string()));
    }
}
