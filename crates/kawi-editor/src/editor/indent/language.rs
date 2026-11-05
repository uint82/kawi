#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndentRules {
    pub indent_after_suffixes: Vec<String>,
    pub indent_after_keywords: Vec<String>,
    pub indent_brackets: bool,
    pub indent_tags: bool,
}

impl Default for IndentRules {
    fn default() -> Self {
        Self {
            indent_after_suffixes: Vec::new(),
            indent_after_keywords: Vec::new(),
            indent_brackets: true,
            indent_tags: false,
        }
    }
}

impl IndentRules {
    pub fn new(
        indent_after_suffixes: Vec<String>,
        indent_after_keywords: Vec<String>,
        indent_brackets: bool,
    ) -> Self {
        Self {
            indent_after_suffixes,
            indent_after_keywords,
            indent_brackets,
            indent_tags: false,
        }
    }

    pub fn with_indent_tags(mut self, indent_tags: bool) -> Self {
        self.indent_tags = indent_tags;
        self
    }
}
