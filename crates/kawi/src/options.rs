use kawi_editor::{DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT};
use kawi_lsp::MAX_SHIFTWIDTH;
use kawi_tui::theme::Theme;
use std::path::Path;

use crate::keymap::flavor_from_name;

const MAX_NUMBERWIDTH: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionScope {
    Global,
    Buffer,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigSection {
    Ui,
    Editor,
    History,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum OptionKind {
    Bool,
    Int { min: usize, max: usize },
    Enum(fn(&str, &OptionContext<'_>) -> bool),
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OptionContext<'a> {
    pub(crate) theme_dir: Option<&'a Path>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionDefault {
    Bool(bool),
    Int(usize),
    Text(&'static str),
}

impl OptionDefault {
    pub(crate) fn value(self) -> OptionValue {
        match self {
            Self::Bool(value) => OptionValue::Bool(value),
            Self::Int(value) => OptionValue::Int(value),
            Self::Text(value) => OptionValue::Text(value.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OptionValue {
    Bool(bool),
    Int(usize),
    Text(String),
}

impl OptionValue {
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_int(&self) -> Option<usize> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn display(&self) -> String {
        match self {
            Self::Bool(true) => "true".to_string(),
            Self::Bool(false) => "false".to_string(),
            Self::Int(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OptionSpec {
    pub(crate) name: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) kind: OptionKind,
    pub(crate) scope: OptionScope,
    pub(crate) section: ConfigSection,
    pub(crate) config_key: &'static str,
    pub(crate) default: OptionDefault,
}

impl OptionSpec {
    pub(crate) fn default_value(self) -> OptionValue {
        self.default.value()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OptionInput<'a> {
    Bool(bool),
    Integer(i64),
    Text(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OptionError {
    InvalidType {
        expected: &'static str,
        value: String,
    },
    OutOfRange {
        value: i64,
        min: usize,
        max: usize,
    },
    InvalidValue {
        value: String,
    },
}

pub(crate) const OPTIONS: [OptionSpec; 11] = [
    OptionSpec {
        name: "keymap",
        aliases: &["kmp"],
        kind: OptionKind::Enum(valid_keymap),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "keymap",
        default: OptionDefault::Text("vim"),
    },
    OptionSpec {
        name: "theme",
        aliases: &[],
        kind: OptionKind::Enum(valid_theme),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "theme",
        default: OptionDefault::Text("default"),
    },
    OptionSpec {
        name: "shiftwidth",
        aliases: &["sw"],
        kind: OptionKind::Int {
            min: 0,
            max: MAX_SHIFTWIDTH,
        },
        scope: OptionScope::Buffer,
        section: ConfigSection::Editor,
        config_key: "shiftwidth",
        default: OptionDefault::Int(8),
    },
    OptionSpec {
        name: "expandtab",
        aliases: &["et"],
        kind: OptionKind::Bool,
        scope: OptionScope::Buffer,
        section: ConfigSection::Editor,
        config_key: "expandtab",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "number",
        aliases: &["nu"],
        kind: OptionKind::Bool,
        scope: OptionScope::Window,
        section: ConfigSection::Editor,
        config_key: "number",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "relativenumber",
        aliases: &["rnu"],
        kind: OptionKind::Bool,
        scope: OptionScope::Window,
        section: ConfigSection::Editor,
        config_key: "relativenumber",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "numberwidth",
        aliases: &["nuw"],
        kind: OptionKind::Int {
            min: 1,
            max: MAX_NUMBERWIDTH,
        },
        scope: OptionScope::Window,
        section: ConfigSection::Editor,
        config_key: "numberwidth",
        default: OptionDefault::Int(4),
    },
    OptionSpec {
        name: "ignorecase",
        aliases: &["ic"],
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "ignorecase",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "smartcase",
        aliases: &["scs"],
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "smartcase",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "autoindent",
        aliases: &["ai"],
        kind: OptionKind::Bool,
        scope: OptionScope::Buffer,
        section: ConfigSection::Editor,
        config_key: "autoindent",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "history",
        aliases: &[],
        kind: OptionKind::Int {
            min: 0,
            max: MAX_HISTORY_LIMIT,
        },
        scope: OptionScope::Global,
        section: ConfigSection::History,
        config_key: "limit",
        default: OptionDefault::Int(DEFAULT_HISTORY_LIMIT),
    },
];

pub(crate) fn lookup(name: &str) -> Option<&'static OptionSpec> {
    OPTIONS
        .iter()
        .find(|spec| spec.name == name || spec.aliases.contains(&name))
}

pub(crate) fn canonical_name(name: &str) -> Option<&'static str> {
    lookup(name).map(|spec| spec.name)
}

pub(crate) fn validate_spec(
    spec: &'static OptionSpec,
    input: OptionInput<'_>,
    context: &OptionContext<'_>,
) -> Result<OptionValue, OptionError> {
    match spec.kind {
        OptionKind::Bool => parse_bool(input),
        OptionKind::Int { min, max } => parse_int(input, min, max),
        OptionKind::Enum(valid) => parse_enum(input, valid, context),
    }
}

fn parse_bool(input: OptionInput<'_>) -> Result<OptionValue, OptionError> {
    match input {
        OptionInput::Bool(value) => Ok(OptionValue::Bool(value)),
        OptionInput::Text(value) => parse_bool_text(value),
        OptionInput::Integer(value) => Err(OptionError::InvalidType {
            expected: "a boolean",
            value: value.to_string(),
        }),
    }
}

fn parse_bool_text(value: &str) -> Result<OptionValue, OptionError> {
    match value {
        "true" => Ok(OptionValue::Bool(true)),
        "false" => Ok(OptionValue::Bool(false)),
        _ => Err(OptionError::InvalidType {
            expected: "a boolean",
            value: value.to_string(),
        }),
    }
}

fn parse_int(input: OptionInput<'_>, min: usize, max: usize) -> Result<OptionValue, OptionError> {
    let value = match input {
        OptionInput::Integer(value) => value,
        OptionInput::Text(value) => value.parse::<i64>().map_err(|_| OptionError::InvalidType {
            expected: "an integer",
            value: value.to_string(),
        })?,
        OptionInput::Bool(value) => {
            return Err(OptionError::InvalidType {
                expected: "an integer",
                value: value.to_string(),
            })
        }
    };
    if value < min as i64 || value > max as i64 {
        return Err(OptionError::OutOfRange { value, min, max });
    }
    let value = usize::try_from(value).map_err(|_| OptionError::OutOfRange { value, min, max })?;
    Ok(OptionValue::Int(value))
}

fn parse_enum(
    input: OptionInput<'_>,
    valid: fn(&str, &OptionContext<'_>) -> bool,
    context: &OptionContext<'_>,
) -> Result<OptionValue, OptionError> {
    match input {
        OptionInput::Text(value) if valid(value, context) => {
            Ok(OptionValue::Text(value.to_string()))
        }
        OptionInput::Text(value) => Err(OptionError::InvalidValue {
            value: value.to_string(),
        }),
        OptionInput::Bool(value) => Err(OptionError::InvalidType {
            expected: "a name",
            value: value.to_string(),
        }),
        OptionInput::Integer(value) => Err(OptionError::InvalidType {
            expected: "a name",
            value: value.to_string(),
        }),
    }
}

fn valid_keymap(name: &str, _context: &OptionContext<'_>) -> bool {
    flavor_from_name(name).is_some()
}

fn valid_theme(name: &str, context: &OptionContext<'_>) -> bool {
    match context.theme_dir {
        Some(theme_dir) => Theme::load(name, theme_dir).is_ok(),
        None => Theme::named(name).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn names_and_aliases_are_unique() {
        let mut names = BTreeSet::new();
        for spec in OPTIONS {
            assert!(names.insert(spec.name));
            for alias in spec.aliases {
                assert!(names.insert(*alias));
            }
        }
    }

    #[test]
    fn resolves_canonical_names_and_aliases() {
        assert_eq!(canonical_name("shiftwidth"), Some("shiftwidth"));
        assert_eq!(canonical_name("sw"), Some("shiftwidth"));
        assert_eq!(canonical_name("number"), Some("number"));
        assert_eq!(canonical_name("nu"), Some("number"));
        assert_eq!(canonical_name("nope"), None);
    }

    #[test]
    fn validates_boolean_values() {
        let spec = lookup("expandtab").unwrap();
        let value =
            validate_spec(spec, OptionInput::Bool(true), &OptionContext::default()).unwrap();
        assert_eq!(value, OptionValue::Bool(true));
        assert_eq!(
            validate_spec(spec, OptionInput::Integer(1), &OptionContext::default()),
            Err(OptionError::InvalidType {
                expected: "a boolean",
                value: "1".to_string(),
            })
        );
    }

    #[test]
    fn validates_integer_ranges() {
        let spec = lookup("numberwidth").unwrap();
        let value =
            validate_spec(spec, OptionInput::Integer(20), &OptionContext::default()).unwrap();
        assert_eq!(value, OptionValue::Int(20));
        assert_eq!(
            validate_spec(spec, OptionInput::Integer(0), &OptionContext::default()),
            Err(OptionError::OutOfRange {
                value: 0,
                min: 1,
                max: 20,
            })
        );
        assert_eq!(
            validate_spec(spec, OptionInput::Integer(21), &OptionContext::default()),
            Err(OptionError::OutOfRange {
                value: 21,
                min: 1,
                max: 20,
            })
        );
    }

    #[test]
    fn validates_dynamic_enums() {
        let keymap = lookup("keymap").unwrap();
        let value = validate_spec(
            keymap,
            OptionInput::Text("helix"),
            &OptionContext::default(),
        )
        .unwrap();
        assert_eq!(value, OptionValue::Text("helix".to_string()));
        assert_eq!(
            validate_spec(
                keymap,
                OptionInput::Text("personal"),
                &OptionContext::default()
            ),
            Err(OptionError::InvalidValue {
                value: "personal".to_string(),
            })
        );
        let theme = lookup("theme").unwrap();
        let value = validate_spec(
            theme,
            OptionInput::Text("gruvbox-dark"),
            &OptionContext::default(),
        )
        .unwrap();
        assert_eq!(value, OptionValue::Text("gruvbox-dark".to_string()));
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(lookup("nope").is_none());
    }

    #[test]
    fn registry_declares_scopes_and_defaults() {
        let scope = |name| lookup(name).unwrap().scope;
        assert_eq!(scope("ignorecase"), OptionScope::Global);
        assert_eq!(scope("smartcase"), OptionScope::Global);
        assert_eq!(scope("expandtab"), OptionScope::Buffer);
        assert_eq!(scope("shiftwidth"), OptionScope::Buffer);
        assert_eq!(scope("autoindent"), OptionScope::Buffer);
        assert_eq!(scope("number"), OptionScope::Window);
        assert_eq!(scope("relativenumber"), OptionScope::Window);
        assert_eq!(scope("numberwidth"), OptionScope::Window);
        assert_eq!(
            lookup("shiftwidth").unwrap().default_value(),
            OptionValue::Int(8)
        );
        assert_eq!(
            lookup("keymap").unwrap().default_value(),
            OptionValue::Text("vim".to_string())
        );
    }

    #[test]
    fn registry_covers_command_completion_names() {
        let names: BTreeSet<&str> = OPTIONS.iter().map(|spec| spec.name).collect();
        let completions: BTreeSet<&str> = kawi_editor::command::SET_OPTION_NAMES
            .iter()
            .copied()
            .collect();
        assert_eq!(names, completions);
    }
}
