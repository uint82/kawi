use kawi_editor::{DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT};
use kawi_lsp::MAX_SHIFTWIDTH;
use kawi_tui::theme::Theme;
use std::path::Path;

const MAX_NUMBERWIDTH: usize = 20;
pub(crate) const HOVER_MAX_WIDTH_DEFAULT: usize = 64;
pub(crate) const DIAGNOSTIC_POPUP_MAX_WIDTH_DEFAULT: usize = 72;
pub(crate) const COMPLETION_DOCS_MAX_WIDTH_DEFAULT: usize = 42;
pub(crate) const OVERLAY_MIN_WIDTH_DEFAULT: usize = 20;
pub(crate) const OVERLAY_MAX_HEIGHT_PERCENT_DEFAULT: usize = 50;
pub(crate) const OVERLAY_MARGIN_DEFAULT: usize = 2;
pub(crate) const EXPLORER_WIDTH_DEFAULT: usize = 30;
pub(crate) const EXPLORER_WIDTH_MIN: usize = 20;
pub(crate) const EXPLORER_WIDTH_MAX: usize = 60;
pub(crate) const DIRECTORY_METADATA_PERMISSIONS: &str = "permissions";
pub(crate) const DIRECTORY_METADATA_SIZE: &str = "size";
pub(crate) const DIRECTORY_METADATA_MODIFIED: &str = "modified";
pub(crate) const DIRECTORY_METADATA_COLUMNS: &[&str] = &[
    DIRECTORY_METADATA_PERMISSIONS,
    DIRECTORY_METADATA_SIZE,
    DIRECTORY_METADATA_MODIFIED,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AutosaveMode {
    #[default]
    Off,
    Idle,
    Focus,
    All,
}

impl AutosaveMode {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "off" => Some(Self::Off),
            "idle" => Some(Self::Idle),
            "focus" => Some(Self::Focus),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Idle => "idle",
            Self::Focus => "focus",
            Self::All => "all",
        }
    }

    pub(crate) fn idle(self) -> bool {
        matches!(self, Self::Idle | Self::All)
    }

    pub(crate) fn focus(self) -> bool {
        matches!(self, Self::Focus | Self::All)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SignColumnMode {
    #[default]
    Never,
    Auto,
    Always,
}

impl SignColumnMode {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "never" => Some(Self::Never),
            "auto" => Some(Self::Auto),
            "always" => Some(Self::Always),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Auto => "auto",
            Self::Always => "always",
        }
    }

    pub(crate) fn is_visible(self, has_signs: bool) -> bool {
        match self {
            Self::Never => false,
            Self::Auto => has_signs,
            Self::Always => true,
        }
    }
}

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
    Explorer,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum OptionKind {
    Bool,
    Int { min: usize, max: usize },
    Enum(OptionValues),
    List(&'static [&'static str]),
    Text,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum OptionValues {
    Static(&'static [&'static str]),
    Themes,
}

impl OptionValues {
    fn accepts(self, value: &str, context: &OptionContext<'_>) -> bool {
        match self {
            Self::Static(values) => values.contains(&value),
            Self::Themes => match context.theme_dir {
                Some(theme_dir) => Theme::validate(value, theme_dir).is_ok(),
                None => Theme::named(value).is_some(),
            },
        }
    }

    fn values(self, context: &OptionContext<'_>) -> Vec<String> {
        match self {
            Self::Static(values) => values.iter().map(|value| (*value).to_string()).collect(),
            Self::Themes => match context.theme_dir {
                Some(theme_dir) => Theme::available(theme_dir),
                None => Theme::names()
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
            },
        }
    }
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
    pub(crate) description: &'static str,
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

    pub(crate) fn values(self, context: &OptionContext<'_>) -> Vec<String> {
        match self.kind {
            OptionKind::Bool => vec!["true".to_string(), "false".to_string()],
            OptionKind::Enum(values) => values.values(context),
            OptionKind::List(values) => values
                .iter()
                .map(|value| (*value).to_string())
                .chain(std::iter::once("none".to_string()))
                .collect(),
            OptionKind::Int { .. } | OptionKind::Text => Vec::new(),
        }
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

pub(crate) const KEYGUIDE_DELAY_DEFAULT: usize = 250;

pub(crate) const OPTIONS: [OptionSpec; 46] = [
    OptionSpec {
        name: "keymap",
        aliases: &["kmp"],
        description: "Editing keymap.",
        kind: OptionKind::Enum(OptionValues::Static(&["vim", "helix"])),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "keymap",
        default: OptionDefault::Text("vim"),
    },
    OptionSpec {
        name: "theme",
        aliases: &[],
        description: "Color theme name.",
        kind: OptionKind::Enum(OptionValues::Themes),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "theme",
        default: OptionDefault::Text("default"),
    },
    OptionSpec {
        name: "shiftwidth",
        aliases: &["sw"],
        description: "Spaces per indentation level.",
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
        description: "Insert spaces instead of tabs.",
        kind: OptionKind::Bool,
        scope: OptionScope::Buffer,
        section: ConfigSection::Editor,
        config_key: "expandtab",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "number",
        aliases: &["nu"],
        description: "Show absolute line numbers.",
        kind: OptionKind::Bool,
        scope: OptionScope::Window,
        section: ConfigSection::Editor,
        config_key: "number",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "relativenumber",
        aliases: &["rnu"],
        description: "Show relative line numbers.",
        kind: OptionKind::Bool,
        scope: OptionScope::Window,
        section: ConfigSection::Editor,
        config_key: "relativenumber",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "numberwidth",
        aliases: &["nuw"],
        description: "Minimum line number width.",
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
        name: "signcolumn",
        aliases: &["scl"],
        description: "Gutter sign column visibility.",
        kind: OptionKind::Enum(OptionValues::Static(&["never", "auto", "always"])),
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "signcolumn",
        default: OptionDefault::Text("never"),
    },
    OptionSpec {
        name: "git_signs",
        aliases: &["git_sign"],
        description: "Show Git changes in the sign column.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "git_signs",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "ignorecase",
        aliases: &["ic"],
        description: "Ignore case in search patterns.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "ignorecase",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "smartcase",
        aliases: &["scs"],
        description: "Use case-sensitive search when the pattern has uppercase.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "smartcase",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "autoindent",
        aliases: &["ai"],
        description: "Continue indentation on new lines.",
        kind: OptionKind::Bool,
        scope: OptionScope::Buffer,
        section: ConfigSection::Editor,
        config_key: "autoindent",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "auto_pairs",
        aliases: &[],
        description: "Insert matching bracket and quote pairs.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "auto_pairs",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "match_brackets",
        aliases: &[],
        description: "Highlight matching brackets.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "match_brackets",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "format_on_save",
        aliases: &[],
        description: "Format the buffer before saving.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "format_on_save",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "autosave",
        aliases: &[],
        description: "Save automatically after changes.",
        kind: OptionKind::Enum(OptionValues::Static(&["off", "idle", "focus", "all"])),
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "autosave",
        default: OptionDefault::Text("off"),
    },
    OptionSpec {
        name: "autosave_delay",
        aliases: &[],
        description: "Idle delay before autosave in milliseconds.",
        kind: OptionKind::Int {
            min: 100,
            max: 60_000,
        },
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "autosave_delay",
        default: OptionDefault::Int(1000),
    },
    OptionSpec {
        name: "history",
        aliases: &[],
        description: "Maximum undo history entries.",
        kind: OptionKind::Int {
            min: 0,
            max: MAX_HISTORY_LIMIT,
        },
        scope: OptionScope::Global,
        section: ConfigSection::History,
        config_key: "limit",
        default: OptionDefault::Int(DEFAULT_HISTORY_LIMIT),
    },
    OptionSpec {
        name: "undo_persist",
        aliases: &[],
        description: "Persist undo history across sessions.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::History,
        config_key: "undo_persist",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "foldmethod",
        aliases: &["fdm"],
        description: "Fold method.",
        kind: OptionKind::Enum(OptionValues::Static(&["manual", "indent"])),
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "foldmethod",
        default: OptionDefault::Text("manual"),
    },
    OptionSpec {
        name: "foldcolumn",
        aliases: &["fdc"],
        description: "Fold marker column visibility.",
        kind: OptionKind::Int {
            min: 0,
            max: kawi_editor::MAX_FOLD_COLUMN,
        },
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "foldcolumn",
        default: OptionDefault::Int(kawi_editor::DEFAULT_FOLD_COLUMN),
    },
    OptionSpec {
        name: "auto_completion",
        aliases: &[],
        description: "Open completion automatically while typing.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "auto_completion",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "signature_help",
        aliases: &[],
        description: "Show signature help while typing arguments.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "signature_help",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "completion_docs_up",
        aliases: &[],
        description: "Key that scrolls completion documentation up.",
        kind: OptionKind::Text,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "completion_docs_up",
        default: OptionDefault::Text("ctrl-b"),
    },
    OptionSpec {
        name: "completion_docs_down",
        aliases: &[],
        description: "Key that scrolls completion documentation down.",
        kind: OptionKind::Text,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "completion_docs_down",
        default: OptionDefault::Text("ctrl-f"),
    },
    OptionSpec {
        name: "hover_max_width",
        aliases: &[],
        description: "Maximum hover popup width.",
        kind: OptionKind::Int { min: 20, max: 200 },
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "hover_max_width",
        default: OptionDefault::Int(HOVER_MAX_WIDTH_DEFAULT),
    },
    OptionSpec {
        name: "completion_docs_max_width",
        aliases: &[],
        description: "Maximum completion documentation width.",
        kind: OptionKind::Int { min: 20, max: 200 },
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "completion_docs_max_width",
        default: OptionDefault::Int(COMPLETION_DOCS_MAX_WIDTH_DEFAULT),
    },
    OptionSpec {
        name: "overlay_min_width",
        aliases: &[],
        description: "Minimum overlay width.",
        kind: OptionKind::Int { min: 8, max: 200 },
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "overlay_min_width",
        default: OptionDefault::Int(OVERLAY_MIN_WIDTH_DEFAULT),
    },
    OptionSpec {
        name: "overlay_max_height_percent",
        aliases: &[],
        description: "Maximum overlay height as terminal percentage.",
        kind: OptionKind::Int { min: 10, max: 100 },
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "overlay_max_height_percent",
        default: OptionDefault::Int(OVERLAY_MAX_HEIGHT_PERCENT_DEFAULT),
    },
    OptionSpec {
        name: "overlay_margin",
        aliases: &[],
        description: "Overlay margin from window edges.",
        kind: OptionKind::Int { min: 0, max: 40 },
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "overlay_margin",
        default: OptionDefault::Int(OVERLAY_MARGIN_DEFAULT),
    },
    OptionSpec {
        name: "diagnostic_popup_max_width",
        aliases: &[],
        description: "Maximum diagnostic popup width.",
        kind: OptionKind::Int { min: 20, max: 200 },
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "diagnostic_popup_max_width",
        default: OptionDefault::Int(DIAGNOSTIC_POPUP_MAX_WIDTH_DEFAULT),
    },
    OptionSpec {
        name: "diagnostic_virtual_text",
        aliases: &[],
        description: "When diagnostic virtual text is shown.",
        kind: OptionKind::Enum(OptionValues::Static(&["off", "cursor_line", "all"])),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "diagnostic_virtual_text",
        default: OptionDefault::Text("all"),
    },
    OptionSpec {
        name: "diagnostic_virtual_severity",
        aliases: &[],
        description: "Minimum severity for diagnostic virtual text.",
        kind: OptionKind::Enum(OptionValues::Static(&[
            "error",
            "warning",
            "information",
            "hint",
        ])),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "diagnostic_virtual_severity",
        default: OptionDefault::Text("warning"),
    },
    OptionSpec {
        name: "diagnostic_source",
        aliases: &[],
        description: "When diagnostic source names are shown.",
        kind: OptionKind::Enum(OptionValues::Static(&["always", "if_many"])),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "diagnostic_source",
        default: OptionDefault::Text("if_many"),
    },
    OptionSpec {
        name: "diagnostic_counts",
        aliases: &[],
        description: "Severity counts shown in the status line.",
        kind: OptionKind::Enum(OptionValues::Static(&["errors_warnings", "all"])),
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "diagnostic_counts",
        default: OptionDefault::Text("errors_warnings"),
    },
    OptionSpec {
        name: "diagnostic_workspace_counts",
        aliases: &[],
        description: "Show workspace diagnostic counts.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "diagnostic_workspace_counts",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "explorer_enabled",
        aliases: &["ex"],
        description: "Enable the project explorer.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "explorer_enabled",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "explorer_show_hidden",
        aliases: &["exh"],
        description: "Show hidden files in the project explorer.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "explorer_show_hidden",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "explorer_width",
        aliases: &["exw"],
        description: "Project explorer width in columns.",
        kind: OptionKind::Int {
            min: EXPLORER_WIDTH_MIN,
            max: EXPLORER_WIDTH_MAX,
        },
        scope: OptionScope::Global,
        section: ConfigSection::Ui,
        config_key: "explorer_width",
        default: OptionDefault::Int(EXPLORER_WIDTH_DEFAULT),
    },
    OptionSpec {
        name: "keyguide",
        aliases: &["kg"],
        description: "Show key sequence guidance.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "keyguide",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "keyguide_delay",
        aliases: &["kgd"],
        description: "Delay before showing key sequence guidance in milliseconds.",
        kind: OptionKind::Int { min: 0, max: 5_000 },
        scope: OptionScope::Global,
        section: ConfigSection::Editor,
        config_key: "keyguide_delay",
        default: OptionDefault::Int(KEYGUIDE_DELAY_DEFAULT),
    },
    OptionSpec {
        name: "directory_show_hidden",
        aliases: &["dsh"],
        description: "Show hidden entries in new directory buffers.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Explorer,
        config_key: "show_hidden",
        default: OptionDefault::Bool(false),
    },
    OptionSpec {
        name: "directory_confirm",
        aliases: &["dcf"],
        description: "Confirm destructive directory buffer changes.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Explorer,
        config_key: "confirm",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "directory_trash",
        aliases: &["dtr"],
        description: "Send deleted directory entries to the trash.",
        kind: OptionKind::Bool,
        scope: OptionScope::Global,
        section: ConfigSection::Explorer,
        config_key: "trash",
        default: OptionDefault::Bool(true),
    },
    OptionSpec {
        name: "directory_sort",
        aliases: &["dsr"],
        description: "Directory buffer sort order.",
        kind: OptionKind::Enum(OptionValues::Static(&["directories", "name"])),
        scope: OptionScope::Global,
        section: ConfigSection::Explorer,
        config_key: "sort",
        default: OptionDefault::Text("directories"),
    },
    OptionSpec {
        name: "directory_metadata",
        aliases: &["dmd"],
        description: "Metadata columns for directory buffers, or none.",
        kind: OptionKind::List(DIRECTORY_METADATA_COLUMNS),
        scope: OptionScope::Global,
        section: ConfigSection::Explorer,
        config_key: "metadata",
        default: OptionDefault::Text("permissions,size,modified"),
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
        OptionKind::Enum(values) => parse_enum(input, values, context),
        OptionKind::List(values) => parse_list(input, values),
        OptionKind::Text => parse_text(input),
    }
}

fn parse_text(input: OptionInput<'_>) -> Result<OptionValue, OptionError> {
    match input {
        OptionInput::Text(value) if !value.trim().is_empty() => {
            Ok(OptionValue::Text(value.to_string()))
        }
        OptionInput::Text(value) => Err(OptionError::InvalidValue {
            value: value.to_string(),
        }),
        OptionInput::Bool(value) => Err(OptionError::InvalidType {
            expected: "text",
            value: value.to_string(),
        }),
        OptionInput::Integer(value) => Err(OptionError::InvalidType {
            expected: "text",
            value: value.to_string(),
        }),
    }
}

fn parse_list(
    input: OptionInput<'_>,
    values: &'static [&'static str],
) -> Result<OptionValue, OptionError> {
    let text = match input {
        OptionInput::Text(value) => value.trim(),
        OptionInput::Bool(value) => {
            return Err(OptionError::InvalidType {
                expected: "a list",
                value: value.to_string(),
            })
        }
        OptionInput::Integer(value) => {
            return Err(OptionError::InvalidType {
                expected: "a list",
                value: value.to_string(),
            })
        }
    };
    if text.eq_ignore_ascii_case("none") {
        return Ok(OptionValue::Text("none".to_string()));
    }
    let mut selected: Vec<&str> = Vec::new();
    for value in text.split(',').map(str::trim) {
        if value.is_empty() || !values.contains(&value) {
            return Err(OptionError::InvalidValue {
                value: text.to_string(),
            });
        }
        if !selected.contains(&value) {
            selected.push(value);
        }
    }
    if selected.is_empty() {
        return Err(OptionError::InvalidValue {
            value: text.to_string(),
        });
    }
    Ok(OptionValue::Text(selected.join(",")))
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
    values: OptionValues,
    context: &OptionContext<'_>,
) -> Result<OptionValue, OptionError> {
    match input {
        OptionInput::Text(value) if values.accepts(value, context) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn explorer_is_opt_in() {
        let spec = lookup("explorer_enabled").expect("explorer option");
        assert_eq!(spec.default_value(), OptionValue::Bool(false));
        assert_eq!(canonical_name("ex"), Some("explorer_enabled"));
    }

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
        assert_eq!(canonical_name("scl"), Some("signcolumn"));
        assert_eq!(canonical_name("git_sign"), Some("git_signs"));
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
        assert!(OPTIONS.iter().all(|spec| !spec.description.is_empty()));
        assert_eq!(scope("ignorecase"), OptionScope::Global);
        assert_eq!(scope("smartcase"), OptionScope::Global);
        assert_eq!(scope("expandtab"), OptionScope::Buffer);
        assert_eq!(scope("shiftwidth"), OptionScope::Buffer);
        assert_eq!(scope("autoindent"), OptionScope::Buffer);
        assert_eq!(scope("auto_pairs"), OptionScope::Global);
        assert_eq!(scope("match_brackets"), OptionScope::Global);
        assert_eq!(scope("number"), OptionScope::Window);
        assert_eq!(scope("relativenumber"), OptionScope::Window);
        assert_eq!(scope("numberwidth"), OptionScope::Window);
        assert_eq!(scope("signcolumn"), OptionScope::Global);
        assert_eq!(scope("git_signs"), OptionScope::Global);
        assert_eq!(
            lookup("signcolumn").unwrap().default_value(),
            OptionValue::Text("never".to_string())
        );
        assert_eq!(
            lookup("git_signs").unwrap().default_value(),
            OptionValue::Bool(false)
        );
        assert_eq!(
            lookup("shiftwidth").unwrap().default_value(),
            OptionValue::Int(8)
        );
        assert_eq!(
            lookup("keymap").unwrap().default_value(),
            OptionValue::Text("vim".to_string())
        );
        assert_eq!(
            lookup("auto_completion").unwrap().default_value(),
            OptionValue::Bool(false)
        );
        assert_eq!(
            lookup("signature_help").unwrap().default_value(),
            OptionValue::Bool(true)
        );
        assert_eq!(
            lookup("completion_docs_up").unwrap().default_value(),
            OptionValue::Text("ctrl-b".to_string())
        );
        assert_eq!(
            lookup("completion_docs_down").unwrap().default_value(),
            OptionValue::Text("ctrl-f".to_string())
        );
    }

    #[test]
    fn option_values_are_accepted() {
        let context = OptionContext::default();
        for spec in &OPTIONS {
            for value in spec.values(&context) {
                assert!(
                    validate_spec(spec, OptionInput::Text(&value), &context).is_ok(),
                    "{}={value}",
                    spec.name
                );
            }
        }
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
