use lsp_types::InsertTextFormat;

pub(super) fn completion_text(value: &str, format: InsertTextFormat) -> String {
    if format == InsertTextFormat::SNIPPET {
        snippet_plain(value)
    } else {
        value.to_string()
    }
}

fn snippet_plain(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '\\' if index + 1 < chars.len() => {
                output.push(chars[index + 1]);
                index += 2;
            }
            '$' => index = append_snippet_dollar(&chars, index, &mut output),
            character => {
                output.push(character);
                index += 1;
            }
        }
    }
    output
}

fn append_snippet_dollar(chars: &[char], index: usize, output: &mut String) -> usize {
    let Some(next) = chars.get(index + 1).copied() else {
        output.push('$');
        return index + 1;
    };
    if next.is_ascii_digit() {
        return skip_while(chars, index + 1, |character| character.is_ascii_digit());
    }
    if next == '{' {
        let Some(end) = matching_brace(chars, index + 1) else {
            return index + 1;
        };
        let body: String = chars[index + 2..end].iter().collect();
        append_placeholder(&body, output);
        return end + 1;
    }
    if next.is_alphanumeric() || next == '_' {
        return skip_while(chars, index + 1, |character| {
            character.is_alphanumeric() || character == '_'
        });
    }
    output.push('$');
    index + 1
}

fn append_placeholder(body: &str, output: &mut String) {
    let value = body.trim_start_matches(|character: char| character.is_ascii_digit());
    if let Some(choice) = value
        .strip_prefix('|')
        .and_then(|rest| rest.split('|').next())
    {
        if let Some(first) = choice.split(',').next() {
            output.push_str(&snippet_plain(first));
        }
        return;
    }
    if let Some((_, default)) = value.split_once(':') {
        output.push_str(&snippet_plain(default));
    }
}

fn matching_brace(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = start;
    while index < chars.len() {
        match chars[index] {
            '\\' => index = index.saturating_add(2),
            '{' => {
                depth = depth.saturating_add(1);
                index += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}

fn skip_while(chars: &[char], start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut index = start;
    while chars
        .get(index)
        .is_some_and(|character| predicate(*character))
    {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_common_snippet_placeholders() {
        assert_eq!(snippet_plain("println!(${1:value})$0"), "println!(value)");
        assert_eq!(snippet_plain("${1|one,two|} rest"), "one rest");
        assert_eq!(snippet_plain("$TM_FILENAME.ts"), ".ts");
        assert_eq!(snippet_plain("literal \\$1"), "literal $1");
    }

    #[test]
    fn keeps_plain_literal_dollars() {
        assert_eq!(snippet_plain("cost $"), "cost $");
        assert_eq!(snippet_plain("cost $$"), "cost $$");
    }
}
