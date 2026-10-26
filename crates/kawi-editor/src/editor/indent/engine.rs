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
    if request.rules.indent_brackets {
        if let Some(opener) = scan.stack.first() {
            return line_width(&request.lines[opener.line], request.tabstop) + request.shiftwidth;
        }
    }
    if scan.stack.is_empty() && matches_block(scan.token(), request.rules) {
        return request.current_width + request.shiftwidth;
    }
    request.current_width
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
}

#[derive(Default)]
struct Scan {
    stack: Vec<Bracket>,
    token: String,
    last_token: String,
    code: bool,
    line_comment: bool,
}

impl Scan {
    fn begin_line(&mut self) {
        self.token.clear();
        self.last_token.clear();
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
