use std::ops::Range;

use super::util::find_bytes;

const COMMENT_START: &[u8] = b"/*";
const COMMENT_END: &[u8] = b"*/";
const QUOTE_SINGLE: u8 = b'\'';
const QUOTE_DOUBLE: u8 = b'"';
const BACKSLASH: u8 = b'\\';
const OPEN_PAREN: u8 = b'(';
const CLOSE_PAREN: u8 = b')';
const OPEN_BRACE: u8 = b'{';
const CLOSE_BRACE: u8 = b'}';
const SEMICOLON: u8 = b';';
const COLON: u8 = b':';

pub(super) fn ranges(text: &str, commit_at_end: bool) -> Vec<Range<usize>> {
    let mut scanner = ValueScanner::new(text.as_bytes());
    scanner.run();
    if commit_at_end {
        scanner.flush_allowed(text.len());
    }
    scanner.ranges
}

struct ValueScanner<'a> {
    bytes: &'a [u8],
    index: usize,
    ranges: Vec<Range<usize>>,
    in_value: bool,
    allowed_start: Option<usize>,
    paren_depth: usize,
    in_comment: bool,
    quote: Option<u8>,
}

impl<'a> ValueScanner<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            index: 0,
            ranges: Vec::new(),
            in_value: false,
            allowed_start: None,
            paren_depth: 0,
            in_comment: false,
            quote: None,
        }
    }

    fn run(&mut self) {
        while self.index < self.bytes.len() {
            if self.in_comment {
                self.step_comment();
            } else if self.quote.is_some() {
                self.step_string();
            } else if self.starts_comment() {
                self.start_comment();
            } else if self.starts_string() {
                self.start_string();
            } else {
                self.step_code();
            }
        }
    }

    fn step_comment(&mut self) {
        match self
            .bytes
            .get(self.index..)
            .and_then(|tail| find_bytes(tail, COMMENT_END, 0))
        {
            Some(offset) => {
                self.index = self
                    .index
                    .saturating_add(offset)
                    .saturating_add(COMMENT_END.len());
                self.in_comment = false;
                self.resume_allowed();
            }
            None => self.index = self.bytes.len(),
        }
    }

    fn step_string(&mut self) {
        match self.bytes.get(self.index).copied() {
            Some(BACKSLASH) => self.index = self.index.saturating_add(2).min(self.bytes.len()),
            Some(byte) if Some(byte) == self.quote => {
                self.quote = None;
                self.index = self.index.saturating_add(1);
                self.resume_allowed();
            }
            Some(_) => self.index = self.index.saturating_add(1),
            None => self.index = self.bytes.len(),
        }
    }

    fn starts_comment(&self) -> bool {
        self.bytes
            .get(self.index..)
            .is_some_and(|tail| tail.starts_with(COMMENT_START))
    }

    fn start_comment(&mut self) {
        self.flush_allowed(self.index);
        self.in_comment = true;
        self.index = self.index.saturating_add(COMMENT_START.len());
    }

    fn starts_string(&self) -> bool {
        matches!(
            self.bytes.get(self.index).copied(),
            Some(QUOTE_SINGLE | QUOTE_DOUBLE)
        )
    }

    fn start_string(&mut self) {
        self.flush_allowed(self.index);
        self.quote = self.bytes.get(self.index).copied();
        self.index = self.index.saturating_add(1);
    }

    fn step_code(&mut self) {
        match self.bytes.get(self.index).copied() {
            Some(BACKSLASH) => self.index = self.index.saturating_add(2).min(self.bytes.len()),
            Some(OPEN_PAREN) => {
                self.paren_depth = self.paren_depth.saturating_add(1);
                self.index = self.index.saturating_add(1);
            }
            Some(CLOSE_PAREN) => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                self.index = self.index.saturating_add(1);
            }
            Some(OPEN_BRACE) => self.discard_value(),
            Some(CLOSE_BRACE | SEMICOLON) => self.commit_value(),
            Some(COLON) => self.start_value(),
            _ => self.index = self.index.saturating_add(1),
        }
    }

    fn discard_value(&mut self) {
        if self.paren_depth == 0 && self.in_value {
            self.in_value = false;
            self.allowed_start = None;
        }
        self.index = self.index.saturating_add(1);
    }

    fn commit_value(&mut self) {
        if self.paren_depth == 0 && self.in_value {
            self.flush_allowed(self.index);
            self.in_value = false;
        }
        self.index = self.index.saturating_add(1);
    }

    fn start_value(&mut self) {
        if !self.in_value {
            self.in_value = true;
            self.allowed_start = Some(self.index.saturating_add(1));
        }
        self.index = self.index.saturating_add(1);
    }

    fn flush_allowed(&mut self, end: usize) {
        if let Some(start) = self.allowed_start.take() {
            if start < end {
                self.ranges.push(start..end);
            }
        }
    }

    fn resume_allowed(&mut self) {
        if self.in_value && self.allowed_start.is_none() {
            self.allowed_start = Some(self.index);
        }
    }
}
