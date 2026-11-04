use super::FormatterConfig;
use crate::editing::{BlockCommentConfig, PairConfig};
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndentConfig {
    pub shiftwidth: Option<usize>,
    pub expandtab: Option<bool>,
    pub tabstop: Option<usize>,
}

impl IndentConfig {
    pub const fn spaces(width: usize) -> Self {
        Self {
            shiftwidth: Some(width),
            expandtab: Some(true),
            tabstop: Some(width),
        }
    }

    pub const fn tabs(width: usize) -> Self {
        Self {
            shiftwidth: Some(width),
            expandtab: Some(false),
            tabstop: Some(width),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LanguageConfig {
    pub name: String,
    pub language_id: String,
    pub grammar: Option<String>,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub root_markers: Vec<String>,
    pub servers: Vec<String>,
    pub formatter: Option<FormatterConfig>,
    pub indent: IndentConfig,
    pub indent_override: IndentConfig,
    pub indent_after_suffixes: Vec<String>,
    pub indent_after_keywords: Vec<String>,
    pub indent_brackets: bool,
    pub line_comment: Option<String>,
    pub block_comment: Option<BlockCommentConfig>,
    pub auto_pairs: Option<Vec<PairConfig>>,
    pub match_pairs: Option<Vec<PairConfig>>,
    pub enabled: bool,
}

impl LanguageConfig {
    pub fn new(name: impl Into<String>, language_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            language_id: language_id.into(),
            grammar: None,
            extensions: Vec::new(),
            filenames: Vec::new(),
            root_markers: Vec::new(),
            servers: Vec::new(),
            formatter: None,
            indent: IndentConfig::default(),
            indent_override: IndentConfig::default(),
            indent_after_suffixes: Vec::new(),
            indent_after_keywords: Vec::new(),
            indent_brackets: true,
            line_comment: None,
            block_comment: None,
            auto_pairs: None,
            match_pairs: None,
            enabled: true,
        }
    }

    pub fn with_line_comment(mut self, token: impl Into<String>) -> Self {
        self.line_comment = Some(token.into());
        self
    }

    pub fn with_block_comment(mut self, open: impl Into<String>, close: impl Into<String>) -> Self {
        self.block_comment = Some(BlockCommentConfig::new(open, close));
        self
    }

    pub fn with_auto_pairs<const N: usize>(mut self, pairs: [(char, char); N]) -> Self {
        self.auto_pairs = Some(
            pairs
                .into_iter()
                .map(|(open, close)| PairConfig::new(open, close))
                .collect(),
        );
        self
    }

    pub fn with_match_pairs<const N: usize>(mut self, pairs: [(char, char); N]) -> Self {
        self.match_pairs = Some(
            pairs
                .into_iter()
                .map(|(open, close)| PairConfig::new(open, close))
                .collect(),
        );
        self
    }

    pub fn with_indent_after_suffixes<const N: usize>(mut self, suffixes: [&str; N]) -> Self {
        self.indent_after_suffixes = suffixes.into_iter().map(str::to_string).collect();
        self
    }

    pub fn with_indent_after_keywords<const N: usize>(mut self, keywords: [&str; N]) -> Self {
        self.indent_after_keywords = keywords.into_iter().map(str::to_string).collect();
        self
    }

    pub fn with_indent_brackets(mut self, indent_brackets: bool) -> Self {
        self.indent_brackets = indent_brackets;
        self
    }

    pub fn with_grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar = Some(grammar.into());
        self
    }

    pub fn with_extensions<const N: usize>(mut self, extensions: [&str; N]) -> Self {
        self.extensions = extensions.into_iter().map(str::to_string).collect();
        self
    }

    pub fn with_root_markers<const N: usize>(mut self, markers: [&str; N]) -> Self {
        self.root_markers = markers.into_iter().map(str::to_string).collect();
        self
    }

    pub fn with_filenames<const N: usize>(mut self, filenames: [&str; N]) -> Self {
        self.filenames = filenames.into_iter().map(str::to_string).collect();
        self
    }

    pub fn with_servers<const N: usize>(mut self, servers: [&str; N]) -> Self {
        self.servers = servers.into_iter().map(str::to_string).collect();
        self
    }

    pub fn with_formatter(mut self, formatter: FormatterConfig) -> Self {
        self.formatter = Some(formatter);
        self
    }

    pub fn with_indent(mut self, indent: IndentConfig) -> Self {
        self.indent = indent;
        self
    }

    pub fn with_spaces(self, width: usize) -> Self {
        self.with_indent(IndentConfig::spaces(width))
    }

    pub fn with_tabs(self, width: usize) -> Self {
        self.with_indent(IndentConfig::tabs(width))
    }

    pub fn matches(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };
        if self.filenames.iter().any(|candidate| candidate == filename) {
            return true;
        }
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            return false;
        };
        self.extensions
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(extension))
    }
}
