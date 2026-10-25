use super::Command;

pub(super) fn parse(text: &str) -> Option<Command> {
    let (command, rest) = split_head(text)?;
    let action = action(command)?;
    let words = split_words(rest)?;
    action(words)
}

type Action = Box<dyn Fn(Vec<String>) -> Option<Command>>;

fn action(command: &str) -> Option<Action> {
    let (force, command) = split_bang(command);
    match command {
        "saveas" | "sav" => Some(save_as(force)),
        "rename" => Some(rename(force)),
        "delete" => Some(delete(force)),
        _ => None,
    }
}

fn save_as(force: bool) -> Action {
    Box::new(move |words| match words.as_slice() {
        [name] => Some(Command::SaveAs {
            name: name.clone(),
            force,
        }),
        _ => Some(Command::Unknown),
    })
}

fn rename(force: bool) -> Action {
    Box::new(move |words| match words.as_slice() {
        [] => Some(Command::RenameFile {
            source: None,
            name: String::new(),
            force,
        }),
        [name] => Some(Command::RenameFile {
            source: None,
            name: name.clone(),
            force,
        }),
        [source, name] => Some(Command::RenameFile {
            source: Some(source.clone()),
            name: name.clone(),
            force,
        }),
        _ => Some(Command::Unknown),
    })
}

fn delete(force: bool) -> Action {
    Box::new(move |words| match words.as_slice() {
        [] => Some(Command::DeleteFile { path: None, force }),
        [path] => Some(Command::DeleteFile {
            path: Some(path.clone()),
            force,
        }),
        _ => Some(Command::Unknown),
    })
}

fn split_bang(command: &str) -> (bool, &str) {
    match command.strip_suffix('!') {
        Some(command) => (true, command),
        None => (false, command),
    }
}

fn split_head(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    Some((&trimmed[..end], trimmed[end..].trim_start()))
}

fn split_words(text: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;
    for character in text.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            started = true;
            continue;
        }
        if character == '\\' {
            escaped = true;
            started = true;
            continue;
        }
        if let Some(open) = quote {
            if character == open {
                quote = None;
            } else {
                current.push(character);
            }
            started = true;
            continue;
        }
        match character {
            '"' | '\'' => {
                quote = Some(character);
                started = true;
            }
            character if character.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            character => {
                current.push(character);
                started = true;
            }
        }
    }
    if quote.is_some() || escaped {
        return None;
    }
    if started {
        words.push(current);
    }
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_quoted_paths_in_one_word() {
        assert_eq!(
            split_words(r#""my file.txt" 'other file.txt'"#).unwrap(),
            vec!["my file.txt", "other file.txt"]
        );
    }

    #[test]
    fn escapes_spaces_and_quotes() {
        assert_eq!(
            split_words(r#"my\ file.txt "a\"b""#).unwrap(),
            vec!["my file.txt", "a\"b"]
        );
    }

    #[test]
    fn rejects_unterminated_quotes() {
        assert!(split_words(r#""unterminated"#).is_none());
    }

    #[test]
    fn parses_file_commands_with_spaces() {
        assert_eq!(
            parse(r#"rename "old name.txt" "new name.txt""#),
            Some(Command::RenameFile {
                source: Some("old name.txt".to_string()),
                name: "new name.txt".to_string(),
                force: false,
            })
        );
        assert_eq!(
            parse(r#"delete! "-dash.txt""#),
            Some(Command::DeleteFile {
                path: Some("-dash.txt".to_string()),
                force: true,
            })
        );
        assert_eq!(
            parse("saveas! 'nested file.txt'"),
            Some(Command::SaveAs {
                name: "nested file.txt".to_string(),
                force: true,
            })
        );
    }

    #[test]
    fn ignores_unrelated_commands() {
        assert!(parse("write").is_none());
    }
}
