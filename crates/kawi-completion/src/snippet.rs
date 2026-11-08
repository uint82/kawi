use std::ops::Range;

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

impl Snippet {
    pub fn parse(source: &str) -> Self {
        parser::Parser::new(source).parse()
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
