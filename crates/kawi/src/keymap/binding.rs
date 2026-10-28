use kawi_editor::Mode;

use super::chord::KeySequence;
use crate::command::CommandId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeyContext {
    Normal,
    Insert,
    Visual,
}

impl KeyContext {
    pub(crate) fn from_mode(mode: Mode) -> Option<Self> {
        match mode {
            Mode::Normal => Some(Self::Normal),
            Mode::Insert | Mode::Replace => Some(Self::Insert),
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => Some(Self::Visual),
            Mode::Command | Mode::Search => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Insert => "insert",
            Self::Visual => "visual",
        }
    }

    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "normal" => Some(Self::Normal),
            "insert" => Some(Self::Insert),
            "visual" => Some(Self::Visual),
            _ => None,
        }
    }
}

#[allow(dead_code)]
// TODO: construct these sources when built-in and plugin bindings are loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BindingSource {
    Default,
    Plugin,
    User,
}

impl BindingSource {
    pub(crate) fn priority(self) -> u8 {
        match self {
            Self::Default => 0,
            Self::Plugin => 1,
            Self::User => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Binding {
    pub(crate) command: CommandId,
    pub(crate) mode: KeyContext,
    pub(crate) keys: KeySequence,
    pub(crate) args: Option<toml::Value>,
    pub(crate) when: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) source: BindingSource,
}

impl Binding {
    pub(crate) fn new(
        command: CommandId,
        mode: KeyContext,
        keys: KeySequence,
        source: BindingSource,
    ) -> Self {
        Self {
            command,
            mode,
            keys,
            args: None,
            when: None,
            description: None,
            source,
        }
    }

    pub(crate) fn with_args(mut self, args: Option<toml::Value>) -> Self {
        self.args = args;
        self
    }

    pub(crate) fn with_when(mut self, when: Option<String>) -> Self {
        self.when = when;
        self
    }

    pub(crate) fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    pub(crate) fn matches(&self, mode: KeyContext, keys: &[super::chord::KeyChord]) -> bool {
        self.mode == mode && self.keys.chords() == keys
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedCommand {
    pub(crate) command: CommandId,
    pub(crate) args: Option<toml::Value>,
    pub(crate) description: Option<String>,
    pub(crate) keys: KeySequence,
}

impl From<Binding> for ResolvedCommand {
    fn from(binding: Binding) -> Self {
        Self {
            command: binding.command,
            args: binding.args,
            description: binding.description,
            keys: binding.keys,
        }
    }
}
