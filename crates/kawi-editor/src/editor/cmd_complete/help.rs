use super::matching;
use crate::command::{CommandCompletion, SET_OPTION_NAMES};

pub(super) fn candidates(
    completion: &CommandCompletion,
    text: &str,
    start: usize,
    pattern: &str,
) -> Option<(usize, Vec<String>)> {
    let args: Vec<&str> = text[..start].split_whitespace().collect();
    match args.as_slice() {
        ["h"] | ["help"] => Some((
            start,
            matching(completion.help_topics().iter().map(String::as_str), pattern),
        )),
        ["h", "option"] | ["help", "option"] => {
            Some((start, matching(SET_OPTION_NAMES.iter().copied(), pattern)))
        }
        _ => None,
    }
}
