use crate::editing::token_is_valid;
use crate::{IndentConfig, LanguageConfig, LanguageWarning, MAX_SHIFTWIDTH};
use toml::Value;

pub(super) struct IndentOverride {
    pub(super) shiftwidth: Option<Value>,
    pub(super) expandtab: Option<Value>,
    pub(super) tabstop: Option<Value>,
}

pub(super) struct RuleOverride {
    pub(super) after_suffixes: Option<Value>,
    pub(super) after_keywords: Option<Value>,
    pub(super) brackets: Option<Value>,
    pub(super) tags: Option<Value>,
}

#[derive(Clone, Copy)]
struct WidthField {
    name: &'static str,
    min: usize,
}

const SHIFTWIDTH_FIELD: WidthField = WidthField {
    name: "shiftwidth",
    min: 0,
};

const TABSTOP_FIELD: WidthField = WidthField {
    name: "tabstop",
    min: 1,
};

pub(super) fn apply(
    indent: &mut IndentConfig,
    override_config: IndentOverride,
    language: &str,
    warnings: &mut Vec<LanguageWarning>,
) {
    if let Some(value) = override_config.shiftwidth {
        apply_shiftwidth(indent, language, value, warnings);
    }
    if let Some(value) = override_config.expandtab {
        apply_expandtab(indent, language, value, warnings);
    }
    if let Some(value) = override_config.tabstop {
        apply_tabstop(indent, language, value, warnings);
    }
}

pub(super) fn apply_rules(
    language: &mut LanguageConfig,
    override_config: RuleOverride,
    warnings: &mut Vec<LanguageWarning>,
) {
    let RuleOverride {
        after_suffixes,
        after_keywords,
        brackets,
        tags,
    } = override_config;
    apply_string_rules(language, after_suffixes, after_keywords, warnings);
    apply_flag_rules(language, brackets, tags, warnings);
}

fn apply_string_rules(
    language: &mut LanguageConfig,
    suffixes: Option<Value>,
    keywords: Option<Value>,
    warnings: &mut Vec<LanguageWarning>,
) {
    if let Some(value) = suffixes {
        apply_string_list(
            &language.name,
            value,
            "indent_after_suffixes",
            &mut language.indent_after_suffixes,
            warnings,
        );
    }
    if let Some(value) = keywords {
        apply_string_list(
            &language.name,
            value,
            "indent_after_keywords",
            &mut language.indent_after_keywords,
            warnings,
        );
    }
}

fn apply_flag_rules(
    language: &mut LanguageConfig,
    brackets: Option<Value>,
    tags: Option<Value>,
    warnings: &mut Vec<LanguageWarning>,
) {
    if let Some(value) = brackets {
        apply_flag(
            &language.name,
            value,
            "indent_brackets",
            &mut language.indent_brackets,
            warnings,
        );
    }
    if let Some(value) = tags {
        apply_flag(
            &language.name,
            value,
            "indent_tags",
            &mut language.indent_tags,
            warnings,
        );
    }
}

fn apply_string_list(
    language_name: &str,
    value: Value,
    field: &str,
    target: &mut Vec<String>,
    warnings: &mut Vec<LanguageWarning>,
) {
    if let Some(tokens) = parse_string_list(language_name, value, field, warnings) {
        *target = tokens;
    }
}

fn parse_string_list(
    language: &str,
    value: Value,
    field: &str,
    warnings: &mut Vec<LanguageWarning>,
) -> Option<Vec<String>> {
    let display = value.to_string();
    let parsed = match value {
        Value::Array(tokens) => tokens
            .iter()
            .map(|token| match token {
                Value::String(text) if token_is_valid(text) => Some(text.clone()),
                _ => None,
            })
            .collect::<Option<Vec<String>>>(),
        _ => None,
    };
    if parsed.is_none() {
        warnings.push(LanguageWarning::new(format!(
            "language {language}: {field} must be an array of non-empty strings, got {display}; keeping the previous rules"
        )));
    }
    parsed
}

fn apply_flag(
    language_name: &str,
    value: Value,
    field: &str,
    target: &mut bool,
    warnings: &mut Vec<LanguageWarning>,
) {
    match value {
        Value::Boolean(value) => *target = value,
        value => warnings.push(LanguageWarning::new(format!(
            "language {language_name}: {field} must be a boolean, got {value}; keeping the previous rule"
        ))),
    }
}

fn parse_width(
    language: &str,
    field: WidthField,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) -> Option<usize> {
    match value {
        Value::Integer(value) if (field.min as i64..=MAX_SHIFTWIDTH as i64).contains(&value) => {
            Some(value as usize)
        }
        value => {
            warnings.push(LanguageWarning::new(format!(
                "language {language}: {} must be an integer from {} to {MAX_SHIFTWIDTH}, got {value}; using editor settings",
                field.name, field.min
            )));
            None
        }
    }
}

fn apply_shiftwidth(
    indent: &mut IndentConfig,
    language: &str,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    indent.shiftwidth = parse_width(language, SHIFTWIDTH_FIELD, value, warnings);
}

fn apply_tabstop(
    indent: &mut IndentConfig,
    language: &str,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    indent.tabstop = parse_width(language, TABSTOP_FIELD, value, warnings);
}

fn apply_expandtab(
    indent: &mut IndentConfig,
    language: &str,
    value: Value,
    warnings: &mut Vec<LanguageWarning>,
) {
    indent.expandtab = match value {
        Value::Boolean(value) => Some(value),
        value => {
            warnings.push(LanguageWarning::new(format!(
                "language {language}: expandtab must be a boolean, got {value}; using editor settings"
            )));
            None
        }
    };
}
