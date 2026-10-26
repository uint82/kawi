use crate::{LanguageConfig, LanguageWarning};
use toml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairConfig {
    pub open: char,
    pub close: char,
}

impl PairConfig {
    pub fn new(open: char, close: char) -> Self {
        Self { open, close }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockCommentConfig {
    pub open: String,
    pub close: String,
}

impl BlockCommentConfig {
    pub fn new(open: impl Into<String>, close: impl Into<String>) -> Self {
        Self {
            open: open.into(),
            close: close.into(),
        }
    }
}

pub(crate) fn apply_line_comment(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    match value {
        Value::String(text) if token_is_valid(&text) => {
            language.line_comment = Some(text);
        }
        Value::String(text) if text.is_empty() => {
            language.line_comment = None;
        }
        value => warnings.push(LanguageWarning::new(format!(
            "language {}: line_comment must be a non-empty single-line string, got {value}; keeping the previous token",
            language.name
        ))),
    }
}

pub(crate) fn apply_block_comment(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    let display = value.to_string();
    let parsed = match &value {
        Value::Array(tokens) if tokens.is_empty() => Some(None),
        Value::Array(tokens) if tokens.len() == 2 => {
            let open = token_string(&tokens[0]);
            let close = token_string(&tokens[1]);
            match (open, close) {
                (Some(open), Some(close)) if token_is_valid(&open) && token_is_valid(&close) => {
                    Some(Some(BlockCommentConfig::new(open, close)))
                }
                _ => None,
            }
        }
        _ => None,
    };
    match parsed {
        Some(tokens) => language.block_comment = tokens,
        None => warnings.push(LanguageWarning::new(format!(
            "language {}: block_comment must be [] or two non-empty single-line strings, got {display}; keeping the previous token",
            language.name
        ))),
    }
}

pub(crate) fn apply_auto_pairs(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    let display = value.to_string();
    let parsed = match &value {
        Value::Array(pairs) => pairs
            .iter()
            .cloned()
            .map(parse_pair)
            .collect::<Option<Vec<PairConfig>>>(),
        _ => None,
    };
    match parsed {
        Some(pairs) => language.auto_pairs = Some(pairs),
        None => warnings.push(LanguageWarning::new(format!(
            "language {}: auto_pairs must be an array of two single-character strings, got {display}; keeping the previous pairs",
            language.name
        ))),
    }
}

pub(crate) fn apply_match_pairs(
    language: &mut LanguageConfig,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    let display = value.to_string();
    let parsed = match &value {
        Value::Array(pairs) => pairs
            .iter()
            .cloned()
            .map(parse_pair)
            .collect::<Option<Vec<PairConfig>>>()
            .filter(|pairs| pairs.iter().all(|pair| pair.open != pair.close)),
        _ => None,
    };
    match parsed {
        Some(pairs) => language.match_pairs = Some(pairs),
        None => warnings.push(LanguageWarning::new(format!(
            "language {}: match_pairs must be an array of two different single-character strings, got {display}; keeping the previous pairs",
            language.name
        ))),
    }
}

fn parse_pair(value: Value) -> Option<PairConfig> {
    let Value::Array(pair) = value else {
        return None;
    };
    if pair.len() != 2 {
        return None;
    }
    let open = pair_char(&pair[0])?;
    let close = pair_char(&pair[1])?;
    Some(PairConfig::new(open, close))
}

fn pair_char(value: &Value) -> Option<char> {
    let text = token_string(value)?;
    let mut chars = text.chars();
    let ch = chars.next()?;
    if chars.next().is_some() || ch.is_whitespace() || ch.is_control() {
        return None;
    }
    Some(ch)
}

fn token_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

pub(crate) fn token_is_valid(token: &str) -> bool {
    !token.trim().is_empty() && !token.contains(['\n', '\r'])
}
