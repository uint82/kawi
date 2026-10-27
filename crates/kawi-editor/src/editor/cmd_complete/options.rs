use super::matching;
use crate::command::{CommandCompletion, SET_OPTION_NAMES};

pub(super) fn candidates(
    completion: &CommandCompletion,
    text: &str,
    start: usize,
    pattern: &str,
) -> Option<(usize, Vec<String>)> {
    if let Some((name, value, value_start)) = assignment(pattern, start) {
        return Some((value_start, value_candidates(completion, name, value)));
    }
    let args: Vec<&str> = text[..start].split_whitespace().collect();
    match args.as_slice() {
        ["set"] => Some((start, option_names(completion, pattern))),
        ["set", name] => Some((start, value_candidates(completion, name, pattern))),
        _ => None,
    }
}

fn assignment(pattern: &str, start: usize) -> Option<(&str, &str, usize)> {
    let (name, value) = pattern.split_once('=')?;
    if name.is_empty() {
        return None;
    }
    Some((
        name,
        value,
        start.saturating_add(name.len()).saturating_add(1),
    ))
}

fn option_names(completion: &CommandCompletion, pattern: &str) -> Vec<String> {
    if pattern.starts_with("no") {
        let names: Vec<String> = completion
            .boolean_options()
            .iter()
            .map(|name| format!("no{name}"))
            .collect();
        return matching(names.iter().map(String::as_str), pattern);
    }
    matching(SET_OPTION_NAMES.iter().copied(), pattern)
}

fn value_candidates(completion: &CommandCompletion, name: &str, pattern: &str) -> Vec<String> {
    matching(
        completion.option_values(name).iter().map(String::as_str),
        pattern,
    )
}
