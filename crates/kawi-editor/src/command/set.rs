use super::{Command, SetValue};

pub(super) fn parse(rest: &[&str]) -> Command {
    let Some(first) = rest.first() else {
        return Command::Unknown;
    };
    if rest.len() > 2 {
        return super::trailing_characters(first);
    }
    match rest {
        [name] => parse_single(name),
        [name, value] => assign(name, value),
        _ => Command::Unknown,
    }
}

fn parse_single(word: &str) -> Command {
    if let Some((name, value)) = word.split_once('=') {
        return assign(name, value);
    }
    if let Some(name) = word.strip_suffix('!') {
        return modifier(name, SetValue::Toggle);
    }
    if let Some(name) = word.strip_suffix('?') {
        return modifier(name, SetValue::Query);
    }
    if let Some(name) = word.strip_suffix('&') {
        return modifier(name, SetValue::Reset);
    }
    if let Some(name) = word.strip_prefix("no").filter(|name| !name.is_empty()) {
        return modifier(name, SetValue::Negate);
    }
    assign(word, "")
}

fn modifier(name: &str, value: SetValue) -> Command {
    if name.is_empty() {
        return Command::Unknown;
    }
    Command::Set {
        name: name.to_string(),
        value,
    }
}

fn assign(name: &str, value: &str) -> Command {
    if name.is_empty() {
        return Command::Unknown;
    }
    let value = if value.is_empty() {
        SetValue::Assign(None)
    } else {
        SetValue::Assign(Some(value.to_string()))
    };
    Command::Set {
        name: name.to_string(),
        value,
    }
}
