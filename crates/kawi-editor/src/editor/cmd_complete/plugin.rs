use super::matching;
use crate::command::CommandCompletion;

pub(super) fn candidates(
    completion: &CommandCompletion,
    text: &str,
    start: usize,
    pattern: &str,
) -> Option<(usize, Vec<String>)> {
    if text == "plugin" {
        return Some((
            text.len(),
            subcommands(completion, pattern)
                .into_iter()
                .map(|name| format!(" {name}"))
                .collect(),
        ));
    }
    let args: Vec<&str> = text[..start].split_whitespace().collect();
    match args.as_slice() {
        ["plugin"] => Some((start, subcommands(completion, pattern))),
        _ => None,
    }
}

fn subcommands(completion: &CommandCompletion, pattern: &str) -> Vec<String> {
    matching(
        completion.plugin_commands().iter().map(String::as_str),
        pattern,
    )
}
