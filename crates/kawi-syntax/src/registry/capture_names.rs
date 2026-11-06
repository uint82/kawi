use std::iter::Peekable;
use std::str::CharIndices;

pub(super) fn capture_names(source: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut characters = source.char_indices().peekable();
    while let Some((_, character)) = characters.next() {
        match character {
            ';' => skip_comment(&mut characters),
            '"' => skip_string(&mut characters),
            '@' => {
                if let Some(name) = scan_capture(source, &mut characters) {
                    names.push(name);
                }
            }
            _ => {}
        }
    }
    names
}

fn skip_comment(characters: &mut Peekable<CharIndices<'_>>) {
    for (_, character) in characters.by_ref() {
        if character == '\n' {
            break;
        }
    }
}

fn skip_string(characters: &mut Peekable<CharIndices<'_>>) {
    let mut is_escaped = false;
    for (_, character) in characters.by_ref() {
        if is_escaped {
            is_escaped = false;
            if character == '\n' {
                break;
            }
            continue;
        }
        match character {
            '\\' => is_escaped = true,
            '"' | '\n' => break,
            _ => {}
        }
    }
}

fn scan_capture<'source>(
    source: &'source str,
    characters: &mut Peekable<CharIndices<'_>>,
) -> Option<&'source str> {
    let (start, first) = *characters.peek()?;
    if !is_capture_start(first) {
        return None;
    }
    let mut end = start + first.len_utf8();
    characters.next();
    while let Some(&(index, character)) = characters.peek() {
        if !is_capture_continue(character) {
            break;
        }
        end = index + character.len_utf8();
        characters.next();
    }
    Some(&source[start..end])
}

fn is_capture_start(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-')
}

fn is_capture_continue(character: char) -> bool {
    is_capture_start(character) || character == '.'
}
