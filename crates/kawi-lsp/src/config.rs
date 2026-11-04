use crate::{defaults, file, LanguageWarning};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

mod language;
mod server;
#[cfg(test)]
mod tests;

pub use language::{IndentConfig, LanguageConfig};
pub use server::{FormatterConfig, ServerConfig};

pub const MAX_SHIFTWIDTH: usize = 1000;

#[derive(Clone)]
pub struct Languages {
    servers: BTreeMap<String, ServerConfig>,
    languages: Vec<LanguageConfig>,
    available: BTreeMap<String, PathBuf>,
    user_servers: BTreeSet<String>,
    warnings: Vec<LanguageWarning>,
}

impl Languages {
    pub fn load(path: &Path) -> Self {
        let (user, mut warnings) = file::load(path);
        let user_servers = user.server.keys().cloned().collect();
        let servers = merge_servers(user.server);
        let mut languages = merge_languages(user.language, &mut warnings);
        normalize_languages(&mut languages);
        warnings.extend(validate(&servers, &languages));
        let available = available_servers(&servers);
        Self {
            servers,
            languages,
            available,
            user_servers,
            warnings,
        }
    }

    pub fn warnings(&self) -> &[LanguageWarning] {
        &self.warnings
    }

    pub fn server_available(&self, name: &str) -> bool {
        self.available.contains_key(name)
    }

    pub fn server(&self, name: &str) -> Option<&ServerConfig> {
        self.servers.get(name)
    }

    pub fn servers(&self) -> impl Iterator<Item = (&str, &ServerConfig)> {
        self.servers
            .iter()
            .map(|(name, server)| (name.as_str(), server))
    }

    pub fn server_path(&self, name: &str) -> Option<&Path> {
        self.available.get(name).map(PathBuf::as_path)
    }

    pub fn refresh_availability(&mut self) {
        self.available = available_servers(&self.servers);
    }

    pub fn server_is_used(&self, name: &str) -> bool {
        self.languages_using(name).next().is_some()
    }

    pub fn server_is_user_configured(&self, name: &str) -> bool {
        self.user_servers.contains(name)
    }

    pub fn languages_using<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a LanguageConfig> + 'a {
        self.languages.iter().filter(move |language| {
            language.enabled && language.servers.iter().any(|server| server == name)
        })
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

    pub fn formatter_for_path(&self, path: &Path) -> Option<&FormatterConfig> {
        self.language_for_path(path)?.formatter.as_ref()
    }

    pub fn formatters(&self) -> impl Iterator<Item = (&LanguageConfig, &FormatterConfig)> {
        self.languages
            .iter()
            .filter(|language| language.enabled)
            .filter_map(|language| {
                language
                    .formatter
                    .as_ref()
                    .map(|formatter| (language, formatter))
            })
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
            user_servers: BTreeSet::new(),
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

fn merge_languages(
    overrides: Vec<file::LanguageOverride>,
    warnings: &mut Vec<LanguageWarning>,
) -> Vec<LanguageConfig> {
    let mut languages = defaults::languages();
    for language in overrides {
        let index = languages
            .iter()
            .position(|candidate| candidate.name == language.name);
        let (merged, language_warnings) = match index {
            Some(index) => {
                let base = languages.remove(index);
                language.overlay(base)
            }
            None => language.into_config(),
        };
        warnings.extend(language_warnings);
        match index {
            Some(index) => languages.insert(index, merged),
            None => languages.push(merged),
        };
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
            kawi_process::find_executable(&server.command).map(|path| (name.clone(), path))
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
        if language
            .formatter
            .as_ref()
            .is_some_and(|formatter| formatter.command.trim().is_empty())
        {
            warnings.push(LanguageWarning::new(format!(
                "formatter for {} has an empty command",
                language.name
            )));
        }
    }
    warnings
}
