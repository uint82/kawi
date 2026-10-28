pub(super) fn sanitize(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            skip_escape(&mut chars);
        } else if character.is_control() {
            output.push(' ');
        } else {
            output.push(character);
        }
    }
    output
}

fn skip_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match chars.peek().copied() {
        Some('[') => {
            chars.next();
            for character in chars.by_ref() {
                if ('@'..='~').contains(&character) {
                    break;
                }
            }
        }
        Some(']') => {
            chars.next();
            while let Some(character) = chars.next() {
                if character == '\u{7}' {
                    break;
                }
                if character == '\u{1b}' && chars.peek() == Some(&'\\') {
                    chars.next();
                    break;
                }
            }
        }
        Some(_) => {
            chars.next();
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_control_characters_and_escape_sequences() {
        assert_eq!(sanitize("safe\u{1b}[31mred\u{1b}[0m"), "safered");
        assert_eq!(sanitize("a\u{7}b\nc"), "a b c");
        assert_eq!(sanitize("\u{1b}]0;title\u{7}text"), "text");
    }
}
