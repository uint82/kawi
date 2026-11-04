use std::collections::BTreeMap;
use toml::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub initialization_options: Option<Value>,
    pub settings: Option<Value>,
    pub enabled: bool,
}

impl ServerConfig {
    pub fn command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            ..Self::default()
        }
    }

    pub fn with_args<const N: usize>(mut self, args: [&str; N]) -> Self {
        self.args = args.into_iter().map(str::to_string).collect();
        self
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            initialization_options: None,
            settings: None,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormatterConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub stdin: bool,
}

impl FormatterConfig {
    pub fn command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            ..Self::default()
        }
    }

    pub fn with_args<const N: usize>(mut self, args: [&str; N]) -> Self {
        self.args = args.into_iter().map(str::to_string).collect();
        self
    }
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            stdin: true,
        }
    }
}
