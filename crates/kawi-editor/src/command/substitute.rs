#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstituteCommand {
    pub range: Option<SubstituteRange>,
    pub form: SubstituteForm,
    pub flags: SubstituteFlags,
    pub count: Option<usize>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubstituteRange {
    Single(SubstituteAddress),
    Pair {
        first: SubstituteAddress,
        second: SubstituteAddress,
        semicolon: bool,
    },
    WholeFile,
    Visual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubstituteAddress {
    Current,
    Last,
    Line(usize),
    Relative(isize),
    WithOffset {
        base: Box<SubstituteAddress>,
        offset: isize,
    },
    Mark(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubstituteForm {
    New {
        pattern: String,
        replacement: String,
    },
    Repeat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstituteFlags {
    pub reuse: bool,
    pub ops: Vec<SubstituteFlag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstituteFlag {
    All,
    Ask,
    CountOnly,
    Error,
    LastSearch,
    Print,
    Number,
    List,
    IgnoreCase,
    MatchCase,
}

const SUBSTITUTE_FLAG_STARTERS: &str = "cegriIp|\"";

pub(super) fn parse(text: &str) -> Option<Result<SubstituteCommand, String>> {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    let range = parse_substitute_range(&chars, &mut index)?;
    skip_spaces(&chars, &mut index);
    let name_end = parse_substitute_name(&chars, index)?;
    index = name_end;
    skip_spaces(&chars, &mut index);
    let result = if index >= chars.len() || is_substitute_repeat_start(chars[index]) {
        parse_substitute_repeat(&chars, index)
    } else if chars[index].is_ascii_alphabetic() {
        Err("E146: Regular expressions can't be delimited by letters".to_string())
    } else {
        parse_substitute_new(&chars, index)
    };
    Some(result.map(|(form, flags, count)| SubstituteCommand {
        range,
        form,
        flags,
        count,
        raw: text.to_string(),
    }))
}

pub(crate) struct SubstitutePreviewInput {
    pub range: Option<SubstituteRange>,
    pub pattern: Option<String>,
    pub replacement: Option<String>,
    pub flags: SubstituteFlags,
    pub count: Option<usize>,
}

pub(crate) fn parse_preview(text: &str) -> Option<SubstitutePreviewInput> {
    if text.trim_start().starts_with("se") {
        return None;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    let range = parse_substitute_range(&chars, &mut index)?;
    skip_spaces(&chars, &mut index);
    let name_end = parse_substitute_name(&chars, index)?;
    index = name_end;
    skip_spaces(&chars, &mut index);
    if index >= chars.len() || is_substitute_repeat_start(chars[index]) {
        let (flags, index) = parse_substitute_flags(&chars, index);
        let (count, index) = parse_substitute_count(&chars, index).ok()?;
        if check_substitute_trailing(&chars, index).is_err() {
            return None;
        }
        return Some(SubstitutePreviewInput {
            range,
            pattern: None,
            replacement: None,
            flags,
            count,
        });
    }
    if chars[index].is_ascii_alphabetic() {
        return None;
    }
    let delimiter = chars[index];
    let pattern_start = index + 1;
    let pattern_end = scan_substitute_delimiter(&chars, pattern_start, delimiter);
    let pattern: String = chars[pattern_start..pattern_end].iter().collect();
    let flags = SubstituteFlags {
        reuse: false,
        ops: Vec::new(),
    };
    if pattern_end >= chars.len() {
        return Some(SubstitutePreviewInput {
            range,
            pattern: Some(pattern),
            replacement: None,
            flags,
            count: None,
        });
    }
    let replacement_start = pattern_end + 1;
    let replacement_end = scan_substitute_delimiter(&chars, replacement_start, delimiter);
    let replacement: String = chars[replacement_start..replacement_end].iter().collect();
    if replacement_end >= chars.len() {
        return Some(SubstitutePreviewInput {
            range,
            pattern: Some(pattern),
            replacement: Some(replacement),
            flags,
            count: None,
        });
    }
    let (flags, index) = parse_substitute_flags(&chars, replacement_end + 1);
    let (count, index) = parse_substitute_count(&chars, index).ok()?;
    if check_substitute_trailing(&chars, index).is_err() {
        return None;
    }
    Some(SubstitutePreviewInput {
        range,
        pattern: Some(pattern),
        replacement: Some(replacement),
        flags,
        count,
    })
}

fn parse_substitute_range(chars: &[char], index: &mut usize) -> Option<Option<SubstituteRange>> {
    if chars.get(*index) == Some(&'%') {
        *index += 1;
        return Some(Some(SubstituteRange::WholeFile));
    }
    let first = parse_substitute_address(chars, index);
    let Some(first) = first else {
        if matches!(chars.get(*index), Some(',') | Some(';')) {
            let semicolon = chars[*index] == ';';
            *index += 1;
            skip_spaces(chars, index);
            let second = parse_substitute_address(chars, index)?;
            return Some(Some(SubstituteRange::Pair {
                first: SubstituteAddress::Current,
                second,
                semicolon,
            }));
        }
        return Some(None);
    };
    skip_spaces(chars, index);
    let separator = match chars.get(*index) {
        Some(',') => Some(false),
        Some(';') => Some(true),
        _ => None,
    };
    let Some(semicolon) = separator else {
        return Some(Some(SubstituteRange::Single(first)));
    };
    *index += 1;
    skip_spaces(chars, index);
    let second = parse_substitute_address(chars, index).unwrap_or(SubstituteAddress::Current);
    Some(Some(SubstituteRange::Pair {
        first,
        second,
        semicolon,
    }))
}

fn parse_substitute_address(chars: &[char], index: &mut usize) -> Option<SubstituteAddress> {
    let ch = *chars.get(*index)?;
    let mut address = match ch {
        '.' => {
            *index += 1;
            SubstituteAddress::Current
        }
        '$' => {
            *index += 1;
            SubstituteAddress::Last
        }
        '\'' => {
            let name = *chars.get(*index + 1)?;
            *index += 2;
            SubstituteAddress::Mark(name)
        }
        '+' | '-' => SubstituteAddress::Relative(0),
        c if c.is_ascii_digit() => {
            let start = *index;
            while chars.get(*index).is_some_and(|c| c.is_ascii_digit()) {
                *index += 1;
            }
            let digits: String = chars[start..*index].iter().collect();
            SubstituteAddress::Line(digits.parse().ok()?)
        }
        _ => return None,
    };
    let offset = parse_substitute_offsets(chars, index);
    address = match (address, offset) {
        (SubstituteAddress::Relative(base), offset) => SubstituteAddress::Relative(base + offset),
        (address, 0) => address,
        (address, offset) => SubstituteAddress::WithOffset {
            base: Box::new(address),
            offset,
        },
    };
    Some(address)
}

fn parse_substitute_offsets(chars: &[char], index: &mut usize) -> isize {
    let mut total = 0isize;
    while let Some(ch) = chars.get(*index).copied() {
        let sign = match ch {
            '+' => 1,
            '-' => -1,
            _ => break,
        };
        *index += 1;
        let start = *index;
        while chars.get(*index).is_some_and(|c| c.is_ascii_digit()) {
            *index += 1;
        }
        let magnitude = if start == *index {
            1
        } else {
            let digits: String = chars[start..*index].iter().collect();
            digits.parse::<isize>().unwrap_or(isize::MAX)
        };
        total = total.saturating_add(sign * magnitude);
    }
    total
}

fn parse_substitute_name(chars: &[char], start: usize) -> Option<usize> {
    if !chars.get(start).is_some_and(|c| c.is_ascii_lowercase()) {
        return None;
    }
    let mut end = start;
    while chars.get(end).is_some_and(|c| c.is_ascii_lowercase()) {
        end += 1;
    }
    for candidate in (start + 1..=end).rev() {
        let name: String = chars[start..candidate].iter().collect();
        if name == "s" || (name.starts_with("su") && "substitute".starts_with(&name)) {
            return Some(candidate);
        }
    }
    None
}

fn is_substitute_repeat_start(ch: char) -> bool {
    ch.is_ascii_digit() || SUBSTITUTE_FLAG_STARTERS.contains(ch)
}

type ParsedSubstitute = (SubstituteForm, SubstituteFlags, Option<usize>);

fn parse_substitute_repeat(chars: &[char], index: usize) -> Result<ParsedSubstitute, String> {
    let (flags, index) = parse_substitute_flags(chars, index);
    let (count, index) = parse_substitute_count(chars, index)?;
    check_substitute_trailing(chars, index)?;
    Ok((SubstituteForm::Repeat, flags, count))
}

fn parse_substitute_new(chars: &[char], index: usize) -> Result<ParsedSubstitute, String> {
    let delimiter = chars[index];
    let pattern_start = index + 1;
    let pattern_end = scan_substitute_delimiter(chars, pattern_start, delimiter);
    let pattern: String = chars[pattern_start..pattern_end].iter().collect();
    let mut index = pattern_end;
    let Some(replacement) = parse_substitute_replacement(chars, &mut index, delimiter) else {
        let flags = SubstituteFlags {
            reuse: false,
            ops: Vec::new(),
        };
        return Ok((
            SubstituteForm::New {
                pattern,
                replacement: String::new(),
            },
            flags,
            None,
        ));
    };
    let (flags, index) = parse_substitute_flags(chars, index);
    let (count, index) = parse_substitute_count(chars, index)?;
    check_substitute_trailing(chars, index)?;
    Ok((
        SubstituteForm::New {
            pattern,
            replacement,
        },
        flags,
        count,
    ))
}

fn parse_substitute_replacement(
    chars: &[char],
    index: &mut usize,
    delimiter: char,
) -> Option<String> {
    if chars.get(*index) != Some(&delimiter) {
        return None;
    }
    *index += 1;
    let start = *index;
    let end = scan_substitute_delimiter(chars, start, delimiter);
    let replacement: String = chars[start..end].iter().collect();
    *index = end;
    if chars.get(*index) == Some(&delimiter) {
        *index += 1;
    }
    Some(replacement)
}

fn scan_substitute_delimiter(chars: &[char], start: usize, delimiter: char) -> usize {
    let mut index = start;
    while index < chars.len() {
        if chars[index] == '\\' && index + 1 < chars.len() {
            index += 2;
            continue;
        }
        if chars[index] == delimiter {
            break;
        }
        index += 1;
    }
    index
}

fn parse_substitute_flags(chars: &[char], index: usize) -> (SubstituteFlags, usize) {
    let mut flags = SubstituteFlags {
        reuse: chars.get(index) == Some(&'&'),
        ops: Vec::new(),
    };
    let mut index = if flags.reuse { index + 1 } else { index };
    while let Some(ch) = chars.get(index).copied() {
        let flag = match ch {
            'g' => SubstituteFlag::All,
            'c' => SubstituteFlag::Ask,
            'n' => SubstituteFlag::CountOnly,
            'e' => SubstituteFlag::Error,
            'r' => SubstituteFlag::LastSearch,
            'p' => SubstituteFlag::Print,
            '#' => SubstituteFlag::Number,
            'l' => SubstituteFlag::List,
            'i' => SubstituteFlag::IgnoreCase,
            'I' => SubstituteFlag::MatchCase,
            _ => break,
        };
        flags.ops.push(flag);
        index += 1;
    }
    (flags, index)
}

fn parse_substitute_count(chars: &[char], index: usize) -> Result<(Option<usize>, usize), String> {
    let mut index = index;
    skip_spaces(chars, &mut index);
    let Some(ch) = chars.get(index).copied() else {
        return Ok((None, index));
    };
    if !ch.is_ascii_digit() {
        return Ok((None, index));
    }
    let start = index;
    while chars.get(index).is_some_and(|c| c.is_ascii_digit()) {
        index += 1;
    }
    let digits: String = chars[start..index].iter().collect();
    let value = digits
        .parse::<u64>()
        .map_err(|_| format!("E1510: Value too large: {digits}"))?;
    if value == 0 {
        return Err("E939: Positive count required".to_string());
    }
    if value > i32::MAX as u64 {
        return Err(format!("E1510: Value too large: {digits}"));
    }
    Ok((Some(value as usize), index))
}

fn check_substitute_trailing(chars: &[char], index: usize) -> Result<(), String> {
    let mut index = index;
    skip_spaces(chars, &mut index);
    let Some(ch) = chars.get(index).copied() else {
        return Ok(());
    };
    if ch == '"' || ch == '|' {
        return Ok(());
    }
    let rest: String = chars[index..].iter().collect();
    Err(format!("E488: Trailing characters: {rest}"))
}

fn skip_spaces(chars: &[char], index: &mut usize) {
    while chars.get(*index).is_some_and(|c| *c == ' ' || *c == '\t') {
        *index += 1;
    }
}

#[cfg(test)]
mod substitute_tests {
    use super::*;
    use crate::command::{parse, Command};

    fn substitute(text: &str) -> SubstituteCommand {
        match parse(text) {
            Command::Substitute(command) => command,
            other => panic!("expected substitute for {text}: {other:?}"),
        }
    }

    fn new_form(text: &str) -> (String, String) {
        match substitute(text).form {
            SubstituteForm::New {
                pattern,
                replacement,
            } => (pattern, replacement),
            SubstituteForm::Repeat => panic!("expected new form for {text}"),
        }
    }

    #[test]
    fn parses_basic_substitute() {
        let command = substitute("s/foo/bar/");
        assert_eq!(command.range, None);
        assert_eq!(
            command.form,
            SubstituteForm::New {
                pattern: "foo".to_string(),
                replacement: "bar".to_string()
            }
        );
        assert!(!command.flags.reuse);
        assert!(command.flags.ops.is_empty());
        assert_eq!(command.count, None);
        assert_eq!(command.raw, "s/foo/bar/");
    }

    #[test]
    fn parses_missing_delimiters() {
        assert_eq!(new_form("s/foo"), ("foo".to_string(), String::new()));
        assert_eq!(
            new_form("s/foo/bar"),
            ("foo".to_string(), "bar".to_string())
        );
        assert_eq!(new_form("s/"), (String::new(), String::new()));
    }

    #[test]
    fn parses_custom_delimiters() {
        assert_eq!(new_form("s#a/b#X#"), ("a/b".to_string(), "X".to_string()));
        assert_eq!(new_form("s,a b,X,"), ("a b".to_string(), "X".to_string()));
        assert_eq!(
            new_form("s/a\\/b/X/"),
            ("a\\/b".to_string(), "X".to_string())
        );
    }

    #[test]
    fn parses_command_name_abbreviations() {
        assert_eq!(new_form("su/foo/X/").0, "foo");
        assert_eq!(new_form("sub/foo/X/").0, "foo");
        assert_eq!(new_form("substitute/foo/X/").0, "foo");
        assert_eq!(substitute("su").form, SubstituteForm::Repeat);
    }

    #[test]
    fn parses_ranges() {
        assert_eq!(
            substitute("%s/foo/X/").range,
            Some(SubstituteRange::WholeFile)
        );
        assert_eq!(
            substitute("1,2s/foo/X/").range,
            Some(SubstituteRange::Pair {
                first: SubstituteAddress::Line(1),
                second: SubstituteAddress::Line(2),
                semicolon: false
            })
        );
        assert_eq!(
            substitute("1;+1s/foo/X/").range,
            Some(SubstituteRange::Pair {
                first: SubstituteAddress::Line(1),
                second: SubstituteAddress::Relative(1),
                semicolon: true
            })
        );
        assert_eq!(
            substitute("'<,'>s/foo/X/").range,
            Some(SubstituteRange::Pair {
                first: SubstituteAddress::Mark('<'),
                second: SubstituteAddress::Mark('>'),
                semicolon: false
            })
        );
        assert_eq!(
            substitute(".-1s/foo/X/").range,
            Some(SubstituteRange::Single(SubstituteAddress::WithOffset {
                base: Box::new(SubstituteAddress::Current),
                offset: -1
            }))
        );
        assert_eq!(
            substitute(".+2s/foo/X/").range,
            Some(SubstituteRange::Single(SubstituteAddress::WithOffset {
                base: Box::new(SubstituteAddress::Current),
                offset: 2
            }))
        );
        assert_eq!(
            substitute("+2s/foo/X/").range,
            Some(SubstituteRange::Single(SubstituteAddress::Relative(2)))
        );
        assert_eq!(
            substitute("$+1s/foo/X/").range,
            Some(SubstituteRange::Single(SubstituteAddress::WithOffset {
                base: Box::new(SubstituteAddress::Last),
                offset: 1
            }))
        );
        assert_eq!(
            substitute("$s/foo/X/").range,
            Some(SubstituteRange::Single(SubstituteAddress::Last))
        );
        assert_eq!(
            substitute("'as/foo/X/").range,
            Some(SubstituteRange::Single(SubstituteAddress::Mark('a')))
        );
        assert_eq!(
            substitute("1, s/foo/X/").range,
            Some(SubstituteRange::Pair {
                first: SubstituteAddress::Line(1),
                second: SubstituteAddress::Current,
                semicolon: false
            })
        );
        assert_eq!(
            substitute(",2s/foo/X/").range,
            Some(SubstituteRange::Pair {
                first: SubstituteAddress::Current,
                second: SubstituteAddress::Line(2),
                semicolon: false
            })
        );
    }

    #[test]
    fn parses_flags() {
        assert_eq!(substitute("s/foo/X/g").flags.ops, vec![SubstituteFlag::All]);
        assert_eq!(
            substitute("s/foo/X/gi").flags.ops,
            vec![SubstituteFlag::All, SubstituteFlag::IgnoreCase]
        );
        assert_eq!(
            substitute("s/foo/X/Ie").flags.ops,
            vec![SubstituteFlag::MatchCase, SubstituteFlag::Error]
        );
        let reused = substitute("s/foo/X/&g");
        assert!(reused.flags.reuse);
        assert_eq!(reused.flags.ops, vec![SubstituteFlag::All]);
        assert_eq!(
            substitute("s/foo/X/np#l").flags.ops,
            vec![
                SubstituteFlag::CountOnly,
                SubstituteFlag::Print,
                SubstituteFlag::Number,
                SubstituteFlag::List
            ]
        );
        assert_eq!(
            substitute("s/foo/X/r").flags.ops,
            vec![SubstituteFlag::LastSearch]
        );
    }

    #[test]
    fn parses_counts() {
        assert_eq!(substitute("s/foo/X/ 2").count, Some(2));
        assert_eq!(substitute("s/foo/X/2").count, Some(2));
        assert_eq!(substitute("s/foo/X/g 3").count, Some(3));
        assert_eq!(substitute("s 4").count, Some(4));
        assert_eq!(substitute("s/foo/X/ ").count, None);
        assert_eq!(
            parse("s/foo/X/ 0"),
            Command::SetError("E939: Positive count required".to_string())
        );
        assert_eq!(
            parse("s/foo/X/ 2147483648"),
            Command::SetError("E1510: Value too large: 2147483648".to_string())
        );
    }

    #[test]
    fn parses_repeat_forms() {
        assert_eq!(substitute("s").form, SubstituteForm::Repeat);
        assert_eq!(substitute("s").range, None);
        assert_eq!(substitute("s").count, None);
        assert_eq!(substitute("s g").flags.ops, vec![SubstituteFlag::All]);
        assert_eq!(substitute("s 3").count, Some(3));
        assert!(substitute("s 3").flags.ops.is_empty());
        assert_eq!(
            substitute("s&"),
            SubstituteCommand {
                range: None,
                form: SubstituteForm::New {
                    pattern: String::new(),
                    replacement: String::new()
                },
                flags: SubstituteFlags {
                    reuse: false,
                    ops: Vec::new()
                },
                count: None,
                raw: "s&".to_string()
            }
        );
    }

    #[test]
    fn rejects_trailing_characters() {
        assert_eq!(
            parse("s/foo/X/z"),
            Command::SetError("E488: Trailing characters: z".to_string())
        );
        assert_eq!(
            parse("s/foo/X/g 2 x"),
            Command::SetError("E488: Trailing characters: x".to_string())
        );
    }

    #[test]
    fn rejects_letter_delimiters() {
        assert_eq!(
            parse("sfoo bar"),
            Command::SetError(
                "E146: Regular expressions can't be delimited by letters".to_string()
            )
        );
    }

    #[test]
    fn ignores_comments_and_command_separators() {
        assert_eq!(
            new_form("s/foo/X/ \" trailing"),
            ("foo".to_string(), "X".to_string())
        );
        assert_eq!(
            new_form("s/foo/X/ | echo"),
            ("foo".to_string(), "X".to_string())
        );
    }

    #[test]
    fn leaves_other_commands_alone() {
        assert_eq!(
            parse("set number"),
            Command::Set {
                name: "number".to_string(),
                value: crate::command::SetValue::Assign(None),
            }
        );
        assert_eq!(parse("w"), Command::Write { force: false });
        assert_eq!(parse("zzz"), Command::Unknown);
    }
}
