#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndentRules {
    pub indent_after_suffixes: Vec<String>,
    pub indent_after_keywords: Vec<String>,
    pub indent_brackets: bool,
    pub indent_tags: bool,
    pub indent_block_openers: Vec<String>,
    pub indent_block_closers: Vec<String>,
    pub indent_align_keywords: Vec<String>,
    pub indent_branch_keywords: Vec<String>,
    pub indent_branch_openers: Vec<String>,
    pub indent_ignore_case: bool,
    pub indent_keywords_in_brackets: bool,
}

impl Default for IndentRules {
    fn default() -> Self {
        Self {
            indent_after_suffixes: Vec::new(),
            indent_after_keywords: Vec::new(),
            indent_brackets: true,
            indent_tags: false,
            indent_block_openers: Vec::new(),
            indent_block_closers: Vec::new(),
            indent_align_keywords: Vec::new(),
            indent_branch_keywords: Vec::new(),
            indent_branch_openers: Vec::new(),
            indent_ignore_case: false,
            indent_keywords_in_brackets: false,
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
            ..Self::default()
        }
    }

    pub fn with_indent_tags(mut self, indent_tags: bool) -> Self {
        self.indent_tags = indent_tags;
        self
    }

    pub fn with_indent_block_openers<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.indent_block_openers = keywords.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_indent_block_closers<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.indent_block_closers = keywords.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_indent_align_keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.indent_align_keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_indent_branch_keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.indent_branch_keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_indent_branch_openers<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.indent_branch_openers = keywords.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_indent_ignore_case(mut self, ignore_case: bool) -> Self {
        self.indent_ignore_case = ignore_case;
        self
    }

    pub fn with_indent_keywords_in_brackets(mut self, enabled: bool) -> Self {
        self.indent_keywords_in_brackets = enabled;
        self
    }

    pub(crate) fn is_block_opener(&self, word: &str) -> bool {
        matches_word(word, &self.indent_block_openers, self.indent_ignore_case)
    }

    pub(crate) fn is_block_closer(&self, word: &str) -> bool {
        matches_word(word, &self.indent_block_closers, self.indent_ignore_case)
    }

    pub(crate) fn is_align_keyword(&self, word: &str) -> bool {
        matches_word(word, &self.indent_align_keywords, self.indent_ignore_case)
    }

    pub(crate) fn is_branch_keyword(&self, word: &str) -> bool {
        matches_word(word, &self.indent_branch_keywords, self.indent_ignore_case)
    }

    pub(crate) fn is_branch_opener(&self, word: &str) -> bool {
        matches_word(word, &self.indent_branch_openers, self.indent_ignore_case)
    }

    pub(crate) fn triggers_dedent(&self, word: &str) -> bool {
        self.is_block_closer(word) || self.is_align_keyword(word) || self.is_branch_keyword(word)
    }

    pub(crate) fn token_matches_keyword(&self, token: &str, keyword: &str) -> bool {
        token == keyword
            || keyword_from_token(token).is_some_and(|word| {
                if self.indent_ignore_case {
                    word.eq_ignore_ascii_case(keyword)
                } else {
                    word == keyword
                }
            })
    }
}

pub(crate) fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn matches_word(word: &str, keywords: &[String], ignore_case: bool) -> bool {
    keywords.iter().any(|keyword| {
        if ignore_case {
            word.eq_ignore_ascii_case(keyword)
        } else {
            word == keyword
        }
    })
}

fn keyword_from_token(token: &str) -> Option<&str> {
    let word = token.trim_matches(|ch: char| !is_word_char(ch));
    (!word.is_empty() && word.chars().all(is_word_char)).then_some(word)
}
