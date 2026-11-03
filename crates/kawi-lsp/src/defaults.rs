use crate::{LanguageConfig, ServerConfig};
use std::collections::BTreeMap;

mod languages;
mod servers;
#[cfg(test)]
mod tests;

pub fn servers() -> BTreeMap<String, ServerConfig> {
    servers::servers()
}

pub fn languages() -> Vec<LanguageConfig> {
    languages::languages()
}
