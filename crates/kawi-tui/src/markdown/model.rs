#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownStyle {
    Normal,
    Strong,
    Emphasis,
    Code,
    Heading,
    Quote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownSpan {
    pub text: String,
    pub style: MarkdownStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownLine {
    pub spans: Vec<MarkdownSpan>,
}

impl MarkdownLine {
    pub fn blank() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDocument {
    pub blocks: Vec<MarkdownBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownBlock {
    Text(Vec<MarkdownLine>),
    Code(MarkdownCodeBlock),
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownCodeBlock {
    pub language: Option<String>,
    pub lines: Vec<MarkdownLine>,
}

impl MarkdownCodeBlock {
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(MarkdownLine::text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(super) fn sanitize(line: &str) -> String {
    line.chars()
        .map(|character| match character {
            '\t' => ' ',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect()
}
