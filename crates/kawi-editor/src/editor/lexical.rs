use kawi_syntax::LanguageId;

use crate::editing::LanguageEditing;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LexState {
    Code,
    LineComment,
    BlockComment(usize),
    Quote(char),
}

pub(crate) struct LexicalContext {
    quotes: Vec<char>,
    line_token: Vec<char>,
    block_open: Vec<char>,
    block_close: Vec<char>,
    char_literals: bool,
}

impl LexicalContext {
    pub(crate) fn with_language(editing: &LanguageEditing, language: Option<LanguageId>) -> Self {
        let tokens = &editing.comments;
        Self {
            quotes: editing
                .pairs
                .iter()
                .filter(|pair| pair.is_quote())
                .map(|pair| pair.open)
                .collect(),
            line_token: tokens
                .line
                .as_ref()
                .map(|token| token.chars().collect())
                .unwrap_or_default(),
            block_open: tokens
                .block
                .as_ref()
                .map(|block| block.open.chars().collect())
                .unwrap_or_default(),
            block_close: tokens
                .block
                .as_ref()
                .map(|block| block.close.chars().collect())
                .unwrap_or_default(),
            char_literals: language == Some(LanguageId::Rust),
        }
    }
}

pub(crate) struct LexicalScanner {
    context: LexicalContext,
    state: LexState,
    escaped: bool,
}

impl LexicalScanner {
    pub(crate) fn new(context: LexicalContext) -> Self {
        Self {
            context,
            state: LexState::Code,
            escaped: false,
        }
    }

    pub(crate) fn in_code(&self) -> bool {
        self.state == LexState::Code
    }

    pub(crate) fn in_line_comment(&self) -> bool {
        self.state == LexState::LineComment
    }

    pub(crate) fn finish_line(&mut self) {
        if matches!(self.state, LexState::LineComment | LexState::Quote(_)) {
            self.state = LexState::Code;
        }
        self.escaped = false;
    }

    pub(crate) fn advance(&mut self, chars: &[char], index: usize) -> bool {
        let was_code = self.in_code();
        self.state = match self.state {
            LexState::Code => self.advance_code(chars, index),
            LexState::LineComment => LexState::LineComment,
            LexState::BlockComment(depth) => self.advance_block(chars, index, depth),
            LexState::Quote(quote) => self.advance_quote(chars, index, quote),
        };
        was_code && self.in_code()
    }

    fn advance_code(&self, chars: &[char], index: usize) -> LexState {
        if matches_token(chars, index, &self.context.line_token) {
            return LexState::LineComment;
        }
        if matches_token(chars, index, &self.context.block_open) {
            return LexState::BlockComment(1);
        }
        let ch = chars[index];
        if ch == '\'' && self.context.char_literals && !is_rust_char_literal(chars, index) {
            return LexState::Code;
        }
        if self.context.quotes.contains(&ch) {
            return LexState::Quote(ch);
        }
        LexState::Code
    }

    fn advance_block(&self, chars: &[char], index: usize, depth: usize) -> LexState {
        if matches_token(chars, index, &self.context.block_close) {
            return match depth.checked_sub(1) {
                Some(remaining) if remaining > 0 => LexState::BlockComment(remaining),
                _ => LexState::Code,
            };
        }
        if matches_token(chars, index, &self.context.block_open) {
            return LexState::BlockComment(depth.saturating_add(1));
        }
        LexState::BlockComment(depth)
    }

    fn advance_quote(&mut self, chars: &[char], index: usize, quote: char) -> LexState {
        if self.escaped {
            self.escaped = false;
            return LexState::Quote(quote);
        }
        if chars[index] == '\\' {
            self.escaped = true;
        } else if chars[index] == quote {
            return LexState::Code;
        }
        LexState::Quote(quote)
    }
}

fn is_rust_char_literal(chars: &[char], index: usize) -> bool {
    if chars.get(index) != Some(&'\'') {
        return false;
    }
    let Some(next) = chars.get(index.saturating_add(1)) else {
        return false;
    };
    if *next == '\'' {
        return false;
    }
    let close = if *next == '\\' {
        char_literal_escape_end(chars, index)
    } else {
        Some(index.saturating_add(2))
    };
    close.is_some_and(|close| chars.get(close) == Some(&'\''))
}

fn char_literal_escape_end(chars: &[char], index: usize) -> Option<usize> {
    let escape = chars.get(index.saturating_add(2))?;
    match escape {
        'u' if chars.get(index.saturating_add(3)) == Some(&'{') => {
            let mut close = index.saturating_add(4);
            while chars.get(close) != Some(&'}') {
                close = close.checked_add(1)?;
            }
            Some(close.saturating_add(1))
        }
        'x' => Some(index.saturating_add(5)),
        _ => Some(index.saturating_add(3)),
    }
}

fn matches_token(chars: &[char], index: usize, token: &[char]) -> bool {
    !token.is_empty()
        && chars
            .get(index..index.saturating_add(token.len()))
            .is_some_and(|slice| slice == token)
}
