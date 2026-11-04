use crate::{IndentConfig, LanguageWarning, MAX_SHIFTWIDTH};
use toml::Value;

pub(super) struct IndentOverride {
    pub(super) shiftwidth: Option<Value>,
    pub(super) expandtab: Option<Value>,
    pub(super) tabstop: Option<Value>,
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
