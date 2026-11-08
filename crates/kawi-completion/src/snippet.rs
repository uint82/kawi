use std::ops::Range;
use std::path::Path;

mod parser;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub(crate) text: String,
    pub(crate) stops: Vec<TabStop>,
    pub(crate) final_stop: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabStop {
    pub(crate) index: usize,
    pub(crate) range: Range<usize>,
    pub(crate) choices: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnippetContext {
    file_name: Option<String>,
    file_stem: Option<String>,
    directory: Option<String>,
    file_path: Option<String>,
    current_line: Option<String>,
    current_word: Option<String>,
    line_index: Option<usize>,
    line_indent: String,
}

impl SnippetContext {
    pub fn new(path: Option<&Path>, text: &str, cursor: usize) -> Self {
        let cursor = cursor.min(text.chars().count());
        let cursor_byte = byte_offset(text, cursor);
        let line_start = text[..cursor_byte]
            .rfind('\n')
            .map_or(0, |index| index.saturating_add(1));
        let line_end = text[cursor_byte..]
            .find('\n')
            .map_or(text.len(), |index| cursor_byte.saturating_add(index));
        let line = &text[line_start..line_end];
        let current_word = word_at(text, line_start, line_end, cursor_byte);
        let line_index = text[..cursor_byte]
            .chars()
            .filter(|character| *character == '\n')
            .count();
        let line_indent = line
            .chars()
            .take_while(|character| matches!(character, ' ' | '\t'))
            .collect();
        Self {
            file_name: path
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned()),
            file_stem: path
                .and_then(Path::file_stem)
                .map(|name| name.to_string_lossy().into_owned()),
            directory: path
                .and_then(Path::parent)
                .map(|parent| parent.to_string_lossy().into_owned()),
            file_path: path.map(|path| path.to_string_lossy().into_owned()),
            current_line: Some(line.to_string()),
            current_word: Some(current_word),
            line_index: Some(line_index),
            line_indent,
        }
    }

    pub(crate) fn resolve(&self, name: &str) -> Option<String> {
        match name {
            "TM_SELECTED_TEXT" => Some(String::new()),
            "TM_FILENAME" => Some(self.file_name.clone().unwrap_or_default()),
            "TM_FILENAME_BASE" => Some(self.file_stem.clone().unwrap_or_default()),
            "TM_DIRECTORY" => Some(self.directory.clone().unwrap_or_default()),
            "TM_FILEPATH" => Some(self.file_path.clone().unwrap_or_default()),
            "TM_CURRENT_LINE" => Some(self.current_line.clone().unwrap_or_default()),
            "TM_CURRENT_WORD" => Some(self.current_word.clone().unwrap_or_default()),
            "TM_LINE_INDEX" => Some(self.line_index.unwrap_or_default().to_string()),
            "TM_LINE_NUMBER" => Some(
                self.line_index
                    .unwrap_or_default()
                    .saturating_add(1)
                    .to_string(),
            ),
            _ => None,
        }
    }

    pub fn line_indent(&self) -> &str {
        &self.line_indent
    }
}

impl Snippet {
    pub fn parse(source: &str) -> Self {
        Self::parse_with_context(source, &SnippetContext::default())
    }

    pub fn parse_with_context(source: &str, context: &SnippetContext) -> Self {
        parser::Parser::new(source, context).parse()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn stops(&self) -> &[TabStop] {
        &self.stops
    }

    pub fn final_stop(&self) -> usize {
        self.final_stop
    }
}

impl TabStop {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub fn choices(&self) -> &[String] {
        &self.choices
    }
}

fn byte_offset(text: &str, cursor: usize) -> usize {
    text.char_indices()
        .nth(cursor)
        .map_or(text.len(), |(index, _)| index)
}

fn word_at(text: &str, line_start: usize, line_end: usize, cursor: usize) -> String {
    let mut before: Vec<char> = text[line_start..cursor]
        .chars()
        .rev()
        .take_while(|character| is_word_character(*character))
        .collect();
    before.reverse();
    let after = text[cursor..line_end]
        .chars()
        .take_while(|character| is_word_character(*character));
    before.into_iter().chain(after).collect()
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
