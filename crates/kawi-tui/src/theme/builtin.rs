use super::Theme;

const DEFAULT_THEME: &str = include_str!("../../themes/default.toml");
const GRUVBOX_DARK: &str = include_str!("../../themes/gruvbox-dark.toml");
const GRUVBOX_LIGHT: &str = include_str!("../../themes/gruvbox-light.toml");
const ONE_DARK: &str = include_str!("../../themes/one-dark.toml");

#[cfg(test)]
mod contrast_tests;
#[cfg(test)]
mod snapshots;
#[cfg(test)]
mod syntax_tests;
#[cfg(test)]
mod tests;

impl Theme {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default_theme()),
            "gruvbox-dark" => Some(Self::embedded(name, GRUVBOX_DARK)),
            "one-dark" => Some(Self::embedded(name, ONE_DARK)),
            "gruvbox-light" => Some(Self::embedded(name, GRUVBOX_LIGHT)),
            _ => None,
        }
    }

    pub fn names() -> [&'static str; 4] {
        ["default", "one-dark", "gruvbox-dark", "gruvbox-light"]
    }

    pub fn default_theme() -> Self {
        Self::embedded("default", DEFAULT_THEME)
    }

    fn embedded(name: &str, source: &str) -> Self {
        Self::from_toml(name, source).expect("embedded theme")
    }
}
