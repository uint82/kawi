use super::{Command, ParseFailure};

#[derive(Clone, Copy)]
enum FileOperation {
    SaveAs,
    Rename,
    Delete,
}

impl FileOperation {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "saveas" | "sav" => Some(Self::SaveAs),
            "rename" => Some(Self::Rename),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

pub(super) fn parse(text: &str) -> Option<Result<Command, ParseFailure>> {
    let (command, rest) = split_head(text)?;
    let (force, base) = split_bang(command);
    let operation = FileOperation::from_name(base)?;
    let Some(words) = split_words(rest) else {
        return Some(Err(ParseFailure::invalid_argument(rest)));
    };
    let result = match operation {
        FileOperation::SaveAs => save_as(command, force, &words),
        FileOperation::Rename => rename(force, &words),
        FileOperation::Delete => delete(force, &words),
    };
    Some(result)
}

fn save_as(command: &str, force: bool, words: &[String]) -> Result<Command, ParseFailure> {
    match words {
        [name] => Ok(Command::SaveAs {
            name: name.clone(),
            force,
        }),
        [] => Err(ParseFailure::missing_argument(command)),
        _ => Err(ParseFailure::invalid_argument(words.join(" "))),
    }
}

fn rename(force: bool, words: &[String]) -> Result<Command, ParseFailure> {
    match words {
        [] => Ok(Command::RenameFile {
            source: None,
            name: String::new(),
            force,
        }),
        [name] => Ok(Command::RenameFile {
            source: None,
            name: name.clone(),
            force,
        }),
        [source, name] => Ok(Command::RenameFile {
            source: Some(source.clone()),
            name: name.clone(),
            force,
        }),
        _ => Err(ParseFailure::invalid_argument(words.join(" "))),
    }
}

fn delete(force: bool, words: &[String]) -> Result<Command, ParseFailure> {
    match words {
        [] => Ok(Command::DeleteFile { path: None, force }),
        [path] => Ok(Command::DeleteFile {
            path: Some(path.clone()),
            force,
        }),
        _ => Err(ParseFailure::invalid_argument(words.join(" "))),
    }
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
    let mut started = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.peek() {
                Some(next) if next.is_whitespace() || matches!(next, '\\' | '"' | '\'') => {
                    current.push(characters.next().expect("peeked character"));
                }
                _ => current.push('\\'),
            }
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
    if quote.is_some() {
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
    fn keeps_windows_path_separators() {
        assert_eq!(
            split_words(r#"C:\Users\me\file.txt"#).unwrap(),
            vec![r#"C:\Users\me\file.txt"#]
        );
        assert_eq!(
            split_words(r#""C:\Users\me\file.txt""#).unwrap(),
            vec![r#"C:\Users\me\file.txt"#]
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
            Some(Ok(Command::RenameFile {
                source: Some("old name.txt".to_string()),
                name: "new name.txt".to_string(),
                force: false,
            }))
        );
        assert_eq!(
            parse(r#"delete! "-dash.txt""#),
            Some(Ok(Command::DeleteFile {
                path: Some("-dash.txt".to_string()),
                force: true,
            }))
        );
        assert_eq!(
            parse("saveas! 'nested file.txt'"),
            Some(Ok(Command::SaveAs {
                name: "nested file.txt".to_string(),
                force: true,
            }))
        );
    }

    #[test]
    fn rejects_invalid_argument_counts() {
        assert_eq!(
            parse("rename a b c"),
            Some(Err(ParseFailure::invalid_argument("a b c")))
        );
        assert_eq!(
            parse("delete a b"),
            Some(Err(ParseFailure::invalid_argument("a b")))
        );
        assert_eq!(
            parse("saveas"),
            Some(Err(ParseFailure::missing_argument("saveas")))
        );
    }

    #[test]
    fn rejects_unterminated_quotes_as_parse_failures() {
        assert_eq!(
            parse(r#"rename "unterminated"#),
            Some(Err(ParseFailure::invalid_argument(r#""unterminated"#)))
        );
    }

    #[test]
    fn ignores_unrelated_commands() {
        assert!(parse("write").is_none());
    }
}
