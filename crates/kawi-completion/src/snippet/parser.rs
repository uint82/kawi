use std::collections::BTreeSet;

use super::{Snippet, TabStop};

pub(super) struct Parser {
    chars: Vec<char>,
    index: usize,
    dollar: usize,
    text: String,
    length: usize,
    stops: Vec<TabStop>,
    seen: BTreeSet<usize>,
    final_stop: Option<usize>,
}

impl Parser {
    pub(super) fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
            dollar: 0,
            text: String::new(),
            length: 0,
            stops: Vec::new(),
            seen: BTreeSet::new(),
            final_stop: None,
        }
    }

    pub(super) fn parse(mut self) -> Snippet {
        self.parse_text(false);
        self.stops.sort_by_key(|stop| stop.index);
        Snippet {
            text: self.text,
            final_stop: self.final_stop.unwrap_or(self.length),
            stops: self.stops,
        }
    }

    fn parse_text(&mut self, inside_placeholder: bool) {
        while let Some(character) = self.peek() {
            if inside_placeholder && character == '}' {
                return;
            }
            match character {
                '\\' => self.parse_escape(),
                '$' => self.parse_dollar(),
                _ => {
                    self.push(character);
                    self.index += 1;
                }
            }
        }
    }

    fn parse_escape(&mut self) {
        match self.peek_at(1) {
            Some(character) => {
                self.push(character);
                self.index += 2;
            }
            None => {
                self.push('\\');
                self.index += 1;
            }
        }
    }

    fn parse_dollar(&mut self) {
        self.dollar = self.index;
        match self.peek_at(1) {
            Some(character) if character.is_ascii_digit() => {
                self.index += 1;
                let number = self.parse_number();
                self.record_stop(number, self.length, self.length, Vec::new());
            }
            Some('{') => self.parse_braced(),
            Some(character) if is_variable(character) => self.skip_variable(),
            _ => {
                self.push('$');
                self.index += 1;
            }
        }
    }

    fn parse_braced(&mut self) {
        self.index += 2;
        match self.peek() {
            Some(character) if character.is_ascii_digit() => self.parse_braced_stop(),
            Some(character) if is_variable(character) => self.parse_braced_variable(),
            _ => self.flush_raw(),
        }
    }

    fn parse_braced_stop(&mut self) {
        let index = self.parse_number();
        match self.peek() {
            Some('}') => {
                self.index += 1;
                self.record_stop(index, self.length, self.length, Vec::new());
            }
            Some(':') => {
                self.index += 1;
                let start = self.length;
                self.parse_text(true);
                let end = self.length;
                self.consume_brace();
                self.record_stop(index, start, end, Vec::new());
            }
            Some('|') => {
                self.index += 1;
                let start = self.length;
                let choices = self.parse_choices();
                let end = self.length;
                self.consume_brace();
                self.record_stop(index, start, end, choices);
            }
            Some('/') => {
                self.index += 1;
                self.skip_braced_body();
                self.record_stop(index, self.length, self.length, Vec::new());
            }
            _ => self.flush_raw(),
        }
    }

    fn parse_braced_variable(&mut self) {
        self.skip_variable_name();
        match self.peek() {
            Some('}') => self.index += 1,
            Some(':') => {
                self.index += 1;
                self.parse_text(true);
                self.consume_brace();
            }
            Some('/') => {
                self.index += 1;
                self.skip_braced_body();
            }
            _ => self.flush_raw(),
        }
    }

    fn parse_choices(&mut self) -> Vec<String> {
        let mut choices = Vec::new();
        let mut choice = String::new();
        while let Some(character) = self.peek() {
            match character {
                '\\' => self.push_choice_escape(&mut choice),
                ',' => {
                    choices.push(choice);
                    choice = String::new();
                    self.index += 1;
                }
                '|' => {
                    choices.push(choice);
                    self.index += 1;
                    break;
                }
                '}' => {
                    choices.push(choice);
                    break;
                }
                _ => {
                    choice.push(character);
                    self.index += 1;
                }
            }
        }
        self.push_choices(&choices);
        choices
    }

    fn push_choice_escape(&mut self, choice: &mut String) {
        match self.peek_at(1) {
            Some(character) => {
                choice.push(character);
                self.index += 2;
            }
            None => {
                choice.push('\\');
                self.index += 1;
            }
        }
    }

    fn push_choices(&mut self, choices: &[String]) {
        if let Some(first) = choices.first() {
            self.push_str(first);
        }
    }

    fn skip_variable(&mut self) {
        self.index += 1;
        self.skip_variable_name();
    }

    fn skip_variable_name(&mut self) {
        while self.peek().is_some_and(is_variable) {
            self.index += 1;
        }
    }

    fn parse_number(&mut self) -> usize {
        let mut number = 0usize;
        while let Some(character) = self.peek().filter(|character| character.is_ascii_digit()) {
            number = number
                .saturating_mul(10)
                .saturating_add(character as usize - '0' as usize);
            self.index += 1;
        }
        number
    }

    fn record_stop(&mut self, index: usize, start: usize, end: usize, choices: Vec<String>) {
        if index == 0 {
            self.final_stop.get_or_insert(start);
            return;
        }
        if self.seen.insert(index) {
            self.stops.push(TabStop {
                index,
                range: start..end,
                choices,
            });
        }
    }

    fn consume_brace(&mut self) {
        if self.peek() == Some('}') {
            self.index += 1;
        }
    }

    fn skip_braced_body(&mut self) {
        let mut depth = 1usize;
        while let Some(character) = self.peek() {
            self.index += 1;
            match character {
                '\\' => {
                    if self.peek().is_some() {
                        self.index += 1;
                    }
                }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    fn flush_raw(&mut self) {
        let raw: String = self.chars[self.dollar..self.index].iter().collect();
        self.push_str(&raw);
    }

    fn push(&mut self, character: char) {
        self.text.push(character);
        self.length += 1;
    }

    fn push_str(&mut self, value: &str) {
        self.text.push_str(value);
        self.length += value.chars().count();
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }
}

fn is_variable(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
