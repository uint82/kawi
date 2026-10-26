#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub open: char,
    pub close: char,
}

impl Pair {
    pub const fn new(open: char, close: char) -> Self {
        Self { open, close }
    }

    pub fn is_quote(self) -> bool {
        self.open == self.close && matches!(self.open, '\'' | '"' | '`')
    }

    pub fn contains_closer(self, ch: char) -> bool {
        self.close == ch
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockComment {
    pub open: String,
    pub close: String,
}

impl BlockComment {
    pub fn new(open: impl Into<String>, close: impl Into<String>) -> Self {
        Self {
            open: open.into(),
            close: close.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentTokens {
    pub line: Option<String>,
    pub block: Option<BlockComment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageEditing {
    pub comments: CommentTokens,
    pub pairs: Vec<Pair>,
    pub match_pairs: Vec<Pair>,
}

impl Default for LanguageEditing {
    fn default() -> Self {
        Self {
            comments: CommentTokens::default(),
            pairs: default_pairs(),
            match_pairs: default_match_pairs(),
        }
    }
}

pub fn default_pairs() -> Vec<Pair> {
    vec![
        Pair::new('(', ')'),
        Pair::new('[', ']'),
        Pair::new('{', '}'),
        Pair::new('"', '"'),
        Pair::new('\'', '\''),
        Pair::new('`', '`'),
    ]
}

pub fn default_match_pairs() -> Vec<Pair> {
    vec![
        Pair::new('(', ')'),
        Pair::new('[', ']'),
        Pair::new('{', '}'),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_match_themselves() {
        assert!(Pair::new('"', '"').is_quote());
        assert!(!Pair::new('(', ')').is_quote());
        assert!(!Pair::new('\'', '"').is_quote());
    }

    #[test]
    fn default_pairs_cover_brackets_and_quotes() {
        let pairs = default_pairs();
        assert!(pairs.contains(&Pair::new('(', ')')));
        assert!(pairs.contains(&Pair::new('[', ']')));
        assert!(pairs.contains(&Pair::new('{', '}')));
        assert!(pairs.contains(&Pair::new('"', '"')));
    }

    #[test]
    fn default_match_pairs_match_vim_matchpairs() {
        assert_eq!(
            default_match_pairs(),
            vec![
                Pair::new('(', ')'),
                Pair::new('[', ']'),
                Pair::new('{', '}'),
            ]
        );
    }
}
