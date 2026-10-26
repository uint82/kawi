use crate::editing::LanguageEditing;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LexState {
    Code,
    LineComment,
    BlockComment(usize),
    Quote(char),
}

pub(super) struct LexicalContext {
    quotes: Vec<char>,
    line_token: Vec<char>,
    block_open: Vec<char>,
    block_close: Vec<char>,
}

impl LexicalContext {
    pub(super) fn new(editing: &LanguageEditing) -> Self {
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
        }
    }
}

pub(super) struct LexicalScanner {
    context: LexicalContext,
    state: LexState,
    escaped: bool,
}

impl LexicalScanner {
    pub(super) fn new(context: LexicalContext) -> Self {
        Self {
            context,
            state: LexState::Code,
            escaped: false,
        }
    }

    pub(super) fn in_code(&self) -> bool {
        self.state == LexState::Code
    }

    pub(super) fn finish_line(&mut self) {
        if matches!(self.state, LexState::LineComment | LexState::Quote(_)) {
            self.state = LexState::Code;
        }
        self.escaped = false;
    }

    pub(super) fn advance(&mut self, chars: &[char], index: usize) -> bool {
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
        if self.context.quotes.contains(&chars[index]) {
            return LexState::Quote(chars[index]);
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

fn matches_token(chars: &[char], index: usize, token: &[char]) -> bool {
    !token.is_empty()
        && chars
            .get(index..index.saturating_add(token.len()))
            .is_some_and(|slice| slice == token)
}
