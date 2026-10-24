use crate::{binary, defaults, file, LanguageWarning};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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
pub struct LanguageConfig {
    pub name: String,
    pub language_id: String,
    pub grammar: Option<String>,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub root_markers: Vec<String>,
    pub servers: Vec<String>,
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
            enabled: true,
        }
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

    pub fn with_servers<const N: usize>(mut self, servers: [&str; N]) -> Self {
        self.servers = servers.into_iter().map(str::to_string).collect();
        self
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

#[derive(Clone)]
pub struct Languages {
    servers: BTreeMap<String, ServerConfig>,
    languages: Vec<LanguageConfig>,
    available: BTreeMap<String, PathBuf>,
    warnings: Vec<LanguageWarning>,
}

impl Languages {
    pub fn load(path: &Path) -> Self {
        let (user, mut warnings) = file::load(path);
        let servers = merge_servers(user.server);
        let mut languages = merge_languages(user.language);
        normalize_languages(&mut languages);
        warnings.extend(validate(&servers, &languages));
        let available = available_servers(&servers);
        Self {
            servers,
            languages,
            available,
            warnings,
        }
    }

    pub fn warnings(&self) -> &[LanguageWarning] {
        &self.warnings
    }

    pub fn warning(&self) -> Option<String> {
        (!self.warnings.is_empty()).then(|| {
            self.warnings
                .iter()
                .map(LanguageWarning::message)
                .collect::<Vec<_>>()
                .join("; ")
        })
    }

    pub fn server_available(&self, name: &str) -> bool {
        self.available.contains_key(name)
    }

    pub fn server(&self, name: &str) -> Option<&ServerConfig> {
        self.servers.get(name)
    }

    pub fn missing_binary_status(&self, path: Option<&Path>) -> Option<String> {
        let path = path?;
        let language = self.language_for_path(path)?;
        let missing: Vec<&str> = language
            .servers
            .iter()
            .map(String::as_str)
            .filter(|name| {
                self.servers.get(*name).is_some_and(|server| server.enabled)
                    && !self.available.contains_key(*name)
            })
            .collect();
        if missing.is_empty() {
            return None;
        }
        Some(format!("lsp: {} not found in PATH", missing.join(", ")))
    }

    pub fn language_for_path(&self, path: &Path) -> Option<&LanguageConfig> {
        self.languages
            .iter()
            .find(|language| language.matches(path))
    }

    #[cfg(test)]
    fn from_parts(
        servers: BTreeMap<String, ServerConfig>,
        languages: Vec<LanguageConfig>,
        available: BTreeMap<String, PathBuf>,
    ) -> Self {
        Self {
            servers,
            languages,
            available,
            warnings: Vec::new(),
        }
    }

    #[cfg(test)]
    fn language(&self, name: &str) -> Option<&LanguageConfig> {
        self.languages.iter().find(|language| language.name == name)
    }
}

fn merge_servers(
    overrides: BTreeMap<String, file::ServerOverride>,
) -> BTreeMap<String, ServerConfig> {
    let mut servers = defaults::servers();
    for (name, server) in overrides {
        let base = servers.remove(&name).unwrap_or_default();
        servers.insert(name, server.overlay(base));
    }
    servers
}

fn merge_languages(overrides: Vec<file::LanguageOverride>) -> Vec<LanguageConfig> {
    let mut languages = defaults::languages();
    for language in overrides {
        match languages
            .iter()
            .position(|candidate| candidate.name == language.name)
        {
            Some(index) => {
                let base = languages.remove(index);
                languages.insert(index, language.overlay(base));
            }
            None => languages.push(language.into_config()),
        }
    }
    languages
}

fn normalize_languages(languages: &mut [LanguageConfig]) {
    for language in languages {
        language.extensions = language
            .extensions
            .drain(..)
            .map(|extension| extension.trim_start_matches('.').to_string())
            .collect();
        if language.language_id.is_empty() {
            language.language_id = language.name.clone();
        }
    }
}

fn available_servers(servers: &BTreeMap<String, ServerConfig>) -> BTreeMap<String, PathBuf> {
    servers
        .iter()
        .filter(|(_, server)| server.enabled)
        .filter_map(|(name, server)| {
            binary::find_command(&server.command).map(|path| (name.clone(), path))
        })
        .collect()
}

fn validate(
    servers: &BTreeMap<String, ServerConfig>,
    languages: &[LanguageConfig],
) -> Vec<LanguageWarning> {
    let mut warnings = Vec::new();
    for language in languages.iter().filter(|language| language.enabled) {
        if language.name.is_empty() {
            warnings.push(LanguageWarning::new("language name is empty"));
        }
        for server in &language.servers {
            match servers.get(server) {
                Some(config) if config.command.trim().is_empty() => {
                    warnings.push(LanguageWarning::new(format!(
                        "server {server} for {} has an empty command",
                        language.name
                    )))
                }
                None => warnings.push(LanguageWarning::new(format!(
                    "server {server} for {} is not configured",
                    language.name
                ))),
                _ => {}
            }
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_file(name: &str, text: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_lsp_config_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("languages.toml");
        fs::write(&path, text).expect("write");
        path
    }

    fn server() -> BTreeMap<String, ServerConfig> {
        [(
            "rust-analyzer".to_string(),
            ServerConfig::command("rust-analyzer"),
        )]
        .into_iter()
        .collect()
    }

    fn rust() -> LanguageConfig {
        LanguageConfig::new("rust", "rust")
            .with_extensions(["rs"])
            .with_servers(["rust-analyzer"])
    }

    #[test]
    fn built_in_defaults_cover_rust_and_python() {
        let languages = Languages::load(Path::new("/nonexistent/languages.toml"));
        assert_eq!(
            languages.language("rust").expect("rust").language_id,
            "rust"
        );
        assert_eq!(
            languages.language("python").expect("python").servers,
            vec!["pyright"]
        );
    }

    #[test]
    fn user_languages_preserve_unspecified_defaults() {
        let path = temp_file(
            "override",
            "[[language]]\nname = \"python\"\nservers = [\"pyright\", \"ruff\"]\n",
        );
        let languages = Languages::load(&path);
        let python = languages.language("python").expect("python");
        assert_eq!(python.extensions, vec!["py", "pyi", "pyw"]);
        assert_eq!(python.servers, vec!["pyright", "ruff"]);
    }

    #[test]
    fn user_languages_replace_explicit_fields() {
        let path = temp_file(
            "explicit",
            "[[language]]\nname = \"rust\"\nextensions = [\".rs\"]\nservers = []\n",
        );
        let languages = Languages::load(&path);
        let rust = languages.language("rust").expect("rust");
        assert_eq!(rust.extensions, vec!["rs"]);
        assert!(rust.servers.is_empty());
    }

    #[test]
    fn user_languages_can_be_added() {
        let path = temp_file(
            "new_language",
            "[[language]]\nname = \"lua\"\nextensions = [\"lua\"]\nservers = []\n",
        );
        let languages = Languages::load(&path);
        let lua = languages.language("lua").expect("lua");
        assert_eq!(lua.language_id, "lua");
        assert_eq!(lua.extensions, vec!["lua"]);
    }

    #[test]
    fn user_servers_preserve_unspecified_defaults() {
        let path = temp_file(
            "server_override",
            "[server.pyright]\nargs = [\"--stdio\", \"--verbose\"]\n",
        );
        let languages = Languages::load(&path);
        assert_eq!(
            languages.servers.get("pyright").expect("pyright").command,
            "pyright-langserver"
        );
        assert_eq!(
            languages.servers.get("pyright").expect("pyright").args,
            vec!["--stdio", "--verbose"]
        );
    }

    #[test]
    fn missing_server_references_warn() {
        let path = temp_file(
            "missing_server",
            "[[language]]\nname = \"rust\"\nservers = [\"missing\"]\n",
        );
        let languages = Languages::load(&path);
        assert!(languages
            .warnings()
            .iter()
            .any(|warning| warning.message().contains("server missing")));
    }

    #[test]
    fn invalid_file_falls_back_to_defaults() {
        let path = temp_file("invalid", "history = \n");
        let languages = Languages::load(&path);
        assert_eq!(languages.warnings().len(), 1);
        assert!(languages.warnings()[0].line().is_some());
        assert!(languages.language("rust").is_some());
    }

    #[test]
    fn missing_binary_is_reported_for_the_language() {
        let languages = Languages::from_parts(server(), vec![rust()], BTreeMap::new());
        assert_eq!(
            languages.missing_binary_status(Some(Path::new("src/main.rs"))),
            Some("lsp: rust-analyzer not found in PATH".to_string())
        );
        assert_eq!(
            languages.missing_binary_status(Some(Path::new("notes.txt"))),
            None
        );
    }

    #[test]
    fn available_binary_has_no_status() {
        let mut available = BTreeMap::new();
        available.insert(
            "rust-analyzer".to_string(),
            PathBuf::from("/bin/rust-analyzer"),
        );
        let languages = Languages::from_parts(server(), vec![rust()], available);
        assert_eq!(
            languages.missing_binary_status(Some(Path::new("src/main.rs"))),
            None
        );
    }

    #[test]
    fn optional_ruff_is_reported_when_configured() {
        let mut servers = BTreeMap::new();
        servers.insert(
            "pyright".to_string(),
            ServerConfig::command("pyright-langserver"),
        );
        servers.insert("ruff".to_string(), ServerConfig::command("ruff"));
        let languages = Languages::from_parts(
            servers,
            vec![LanguageConfig::new("python", "python")
                .with_extensions(["py"])
                .with_servers(["pyright", "ruff"])],
            BTreeMap::new(),
        );
        assert_eq!(
            languages.missing_binary_status(Some(Path::new("main.py"))),
            Some("lsp: pyright, ruff not found in PATH".to_string())
        );
    }

    #[test]
    fn disabled_servers_do_not_report_status() {
        let mut servers = server();
        servers.get_mut("rust-analyzer").unwrap().enabled = false;
        let languages = Languages::from_parts(servers, vec![rust()], BTreeMap::new());
        assert_eq!(
            languages.missing_binary_status(Some(Path::new("src/main.rs"))),
            None
        );
    }

    #[test]
    fn pyright_is_the_python_default() {
        let mut servers = server();
        servers.insert(
            "pyright".to_string(),
            ServerConfig::command("pyright-langserver").with_args(["--stdio"]),
        );
        let languages = Languages::from_parts(
            servers,
            vec![LanguageConfig::new("python", "python")
                .with_extensions(["py"])
                .with_servers(["pyright"])],
            BTreeMap::new(),
        );
        assert_eq!(
            languages.missing_binary_status(Some(Path::new("main.py"))),
            Some("lsp: pyright not found in PATH".to_string())
        );
    }
}
