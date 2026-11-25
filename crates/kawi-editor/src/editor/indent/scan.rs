use crate::editor::lexical::{LexicalContext, LexicalScanner};

use super::engine::IndentRequest;
use super::is_word_char;
use super::language::IndentRules;
use super::line_width;
use super::tags::TagScan;

#[derive(Default)]
pub(super) struct Scan {
    pub(super) stack: Vec<Bracket>,
    pub(super) tags: TagScan,
    pub(super) last_bracket: Option<char>,
    pub(super) code: bool,
    pub(super) line_comment: bool,
    pub(super) lines: Vec<LineInfo>,
    pub(super) blocks: Vec<Block>,
    token: String,
    last_token: String,
    word: String,
}

pub(super) struct LineInfo {
    pub(super) indent: usize,
    pub(super) depth: usize,
    pub(super) first_word: String,
    pub(super) last_word: String,
    pub(super) has_code: bool,
}

pub(super) struct Block {
    pub(super) indent: usize,
}

pub(super) struct Bracket {
    pub(super) open: char,
    pub(super) line: usize,
}

pub(super) fn scan(request: &IndentRequest<'_>) -> Scan {
    scan_rows(request, request.row, request.col, false)
}

pub(super) fn scan_before(request: &IndentRequest<'_>) -> Option<Scan> {
    let row = request.row.checked_sub(1)?;
    Some(scan_rows(request, row, usize::MAX, true))
}

fn scan_rows(
    request: &IndentRequest<'_>,
    end_row: usize,
    end_col: usize,
    finish_last: bool,
) -> Scan {
    let mut scanner = LexicalScanner::new(LexicalContext::with_language(
        request.editing,
        request.language,
    ));
    let mut scan = Scan::default();
    for line_index in 0..=end_row {
        let chars: Vec<char> = request.lines[line_index].chars().collect();
        let end = if line_index == end_row {
            end_col.min(chars.len())
        } else {
            chars.len()
        };
        scan.begin_line(line_width(&request.lines[line_index], request.tabstop));
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
        scan.end_line(request.rules);
        if line_index < end_row || finish_last {
            scanner.finish_line();
        }
    }
    scan.code = scanner.in_code();
    scan.line_comment = scanner.in_line_comment();
    scan
}

impl Scan {
    fn begin_line(&mut self, indent: usize) {
        self.token.clear();
        self.last_token.clear();
        self.word.clear();
        self.last_bracket = None;
        self.lines.push(LineInfo {
            indent,
            depth: self.stack.len(),
            first_word: String::new(),
            last_word: String::new(),
            has_code: false,
        });
    }

    fn end_line(&mut self, rules: &IndentRules) {
        self.finish_word();
        let Some(line) = self.lines.last() else {
            return;
        };
        let indent = line.indent;
        let change = if !line.has_code {
            None
        } else if rules.is_block_opener(&line.first_word) {
            Some(BlockChange::Open)
        } else if rules.is_block_closer(&line.first_word) {
            Some(BlockChange::Close)
        } else if line.first_word != line.last_word && rules.is_block_opener(&line.last_word) {
            Some(BlockChange::Open)
        } else {
            None
        };
        match change {
            Some(BlockChange::Open) => self.blocks.push(Block { indent }),
            Some(BlockChange::Close) => {
                self.blocks.pop();
            }
            None => {}
        }
    }

    fn push(&mut self, ch: char, line: usize, previous_code: Option<char>) {
        if ch.is_whitespace() {
            self.finish_word();
            if !self.token.is_empty() {
                self.last_token = std::mem::take(&mut self.token);
            }
        } else {
            self.token.push(ch);
            if is_word_char(ch) {
                self.word.push(ch);
            } else {
                self.finish_word();
            }
        }
        update_brackets(self, ch, line, previous_code);
        self.tags.update(ch, line);
        if let Some(line) = self.lines.last_mut() {
            line.has_code = true;
        }
    }

    fn finish_word(&mut self) {
        if self.word.is_empty() {
            return;
        }
        let word = std::mem::take(&mut self.word);
        let Some(line) = self.lines.last_mut() else {
            return;
        };
        if line.first_word.is_empty() {
            line.first_word = word.clone();
        }
        line.last_word = word;
    }

    pub(super) fn token(&self) -> &str {
        if self.token.is_empty() {
            &self.last_token
        } else {
            &self.token
        }
    }

    pub(super) fn allows_indent(&self) -> bool {
        self.code || self.line_comment
    }
}

enum BlockChange {
    Open,
    Close,
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

pub(super) fn matching_opener(closer: char) -> Option<char> {
    match closer {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        _ => None,
    }
}

pub(super) fn matching_closer(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

pub(crate) fn is_closing_bracket(ch: char) -> bool {
    matching_opener(ch).is_some()
}
