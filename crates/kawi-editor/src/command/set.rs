use super::{Command, ParseFailure, SetValue};

pub(super) fn parse(rest: &[&str]) -> Result<Command, ParseFailure> {
    if rest.is_empty() {
        return Err(ParseFailure::missing_argument("set"));
    }
    if rest.len() > 2 {
        return Err(ParseFailure::trailing_characters(rest[2..].join(" ")));
    }
    match rest {
        [name] => parse_single(name),
        [name, value] => {
            let argument = rest.join(" ");
            require(assign(name, value), &argument)
        }
        _ => unreachable!("rest length is checked before matching"),
    }
}

fn parse_single(word: &str) -> Result<Command, ParseFailure> {
    if let Some((name, value)) = word.split_once('=') {
        return require(assign(name, value), word);
    }
    if let Some(name) = word.strip_suffix('!') {
        return require(modifier(name, SetValue::Toggle), word);
    }
    if let Some(name) = word.strip_suffix('?') {
        return require(modifier(name, SetValue::Query), word);
    }
    if let Some(name) = word.strip_suffix('&') {
        return require(modifier(name, SetValue::Reset), word);
    }
    if let Some(name) = word.strip_prefix("no").filter(|name| !name.is_empty()) {
        return require(modifier(name, SetValue::Negate), word);
    }
    require(assign(word, ""), word)
}

fn require(command: Option<Command>, argument: &str) -> Result<Command, ParseFailure> {
    command.ok_or_else(|| ParseFailure::invalid_argument(argument))
}

fn modifier(name: &str, value: SetValue) -> Option<Command> {
    if name.is_empty() {
        return None;
    }
    Some(Command::Set {
        name: name.to_string(),
        value,
    })
}

fn assign(name: &str, value: &str) -> Option<Command> {
    if name.is_empty() {
        return None;
    }
    let value = if value.is_empty() {
        SetValue::Assign(None)
    } else {
        SetValue::Assign(Some(value.to_string()))
    };
    Some(Command::Set {
        name: name.to_string(),
        value,
    })
}
