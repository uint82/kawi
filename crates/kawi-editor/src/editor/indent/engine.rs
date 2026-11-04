use kawi_buffer::indent_width_of;

use kawi_syntax::LanguageId;

use crate::editing::LanguageEditing;
use crate::editor::lexical::{LexicalContext, LexicalScanner};

use super::language::IndentRules;

pub(crate) const INDENT_WINDOW_LINES: usize = 200;

pub(crate) struct IndentRequest<'a> {
    pub lines: &'a [String],
    pub row: usize,
    pub col: usize,
    pub current_width: usize,
    pub shiftwidth: usize,
    pub tabstop: usize,
    pub rules: &'a IndentRules,
    pub editing: &'a LanguageEditing,
    pub language: Option<LanguageId>,
}

pub(crate) fn next_width(request: &IndentRequest<'_>) -> usize {
    let scan = scan(request);
    if !scan.allows_indent() {
        return request.current_width;
    }
    if scan.stack.is_empty() && matches_block(scan.token(), request.rules) {
        return request.current_width + request.shiftwidth;
    }
    if !request.rules.indent_brackets {
        return request.current_width;
    }
    if scan.last_bracket.is_some_and(is_closing_bracket) {
        return request.current_width;
    }
    let opener = if scan
        .last_bracket
        .is_some_and(|ch| matching_closer(ch).is_some())
    {
        scan.stack.last()
    } else {
        base_opener(&scan)
    };
    opener.map_or(request.current_width, |opener| {
        line_width(&request.lines[opener.line], request.tabstop) + request.shiftwidth
    })
}

fn base_opener(scan: &Scan) -> Option<&Bracket> {
    let brace = scan.stack.iter().rposition(|bracket| bracket.open == '{');
    let start = brace.map_or(0, |index| index + 1);
    scan.stack[start..]
        .iter()
        .find(|bracket| is_continuation_bracket(bracket.open))
        .or_else(|| brace.map(|index| &scan.stack[index]))
}

fn is_continuation_bracket(ch: char) -> bool {
    matches!(ch, '(' | '[')
}

pub(crate) fn closer_width(request: &IndentRequest<'_>, closer: char) -> Option<usize> {
    if !request.rules.indent_brackets {
        return None;
    }
    let scan = scan(request);
    if !scan.code {
        return None;
    }
    let opener = matching_opener(closer)?;
    let bracket = scan.stack.last()?;
    if bracket.open != opener {
        return None;
    }
    Some(line_width(&request.lines[bracket.line], request.tabstop))
}

fn scan(request: &IndentRequest<'_>) -> Scan {
    let mut scanner = LexicalScanner::new(LexicalContext::with_language(
        request.editing,
        request.language,
    ));
    let mut scan = Scan::default();
    for line_index in 0..=request.row {
        let chars: Vec<char> = request.lines[line_index].chars().collect();
        let end = if line_index == request.row {
            request.col.min(chars.len())
        } else {
            chars.len()
        };
        scan.begin_line();
        let mut previous_code = None;
        for index in 0..end {
            if !scanner.advance(&chars, index) {
                previous_code = None;
                continue;
            }
            let ch = chars[index];
            scan.push(ch, line_index, previous_code);
            previous_code = Some(ch);
        }
        if line_index < request.row {
            scanner.finish_line();
        }
    }
    scan.code = scanner.in_code();
    scan.line_comment = scanner.in_line_comment();
    scan
}

fn matches_block(token: &str, rules: &IndentRules) -> bool {
    !token.is_empty()
        && (rules
            .indent_after_suffixes
            .iter()
            .any(|suffix| token.ends_with(suffix))
            || rules
                .indent_after_keywords
                .iter()
                .any(|keyword| token == keyword))
}

fn line_width(line: &str, tabstop: usize) -> usize {
    indent_width_of(line, tabstop.max(1))
}

fn matching_opener(closer: char) -> Option<char> {
    match closer {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        _ => None,
    }
}

pub(crate) fn is_closing_bracket(ch: char) -> bool {
    matching_opener(ch).is_some()
}

fn matching_closer(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

fn update_brackets(scan: &mut Scan, ch: char, line: usize, previous_code: Option<char>) {
    if ch == '{' && previous_code == Some('$') {
        return;
    }
    if matching_closer(ch).is_some() {
        scan.stack.push(Bracket { open: ch, line });
        scan.last_bracket = Some(ch);
        return;
    }
    let Some(opener) = matching_opener(ch) else {
        return;
    };
    if scan
        .stack
        .last()
        .is_some_and(|bracket| bracket.open == opener)
    {
        scan.stack.pop();
    }
    scan.last_bracket = Some(ch);
}

#[derive(Default)]
struct Scan {
    stack: Vec<Bracket>,
    token: String,
    last_token: String,
    last_bracket: Option<char>,
    code: bool,
    line_comment: bool,
}

impl Scan {
    fn begin_line(&mut self) {
        self.token.clear();
        self.last_token.clear();
        self.last_bracket = None;
    }

    fn push(&mut self, ch: char, line: usize, previous_code: Option<char>) {
        if ch.is_whitespace() {
            if !self.token.is_empty() {
                self.last_token = std::mem::take(&mut self.token);
            }
        } else {
            self.token.push(ch);
        }
        update_brackets(self, ch, line, previous_code);
    }

    fn token(&self) -> &str {
        if self.token.is_empty() {
            &self.last_token
        } else {
            &self.token
        }
    }

    fn allows_indent(&self) -> bool {
        self.code || self.line_comment
    }
}

struct Bracket {
    open: char,
    line: usize,
}
