use super::Theme;

mod gruvbox_light;
mod one_dark;
mod palette;

const DEFAULT_THEME: &str = include_str!("../../themes/default.toml");
const GRUVBOX_NIGHT: &str = include_str!("../../themes/gruvbox-night.toml");

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
            "gruvbox-dark" | "gruvbox-night" => Some(Self::gruvbox_night(name)),
            "one-dark" => Some(Self::one_dark()),
            "gruvbox-light" => Some(Self::gruvbox_light()),
            _ => None,
        }
    }

    pub fn names() -> [&'static str; 4] {
        ["default", "one-dark", "gruvbox-dark", "gruvbox-light"]
    }

    pub fn default_theme() -> Self {
        Self::from_toml("default", DEFAULT_THEME).expect("embedded default theme")
    }

    fn gruvbox_night(name: &str) -> Self {
        Self::from_toml(name, GRUVBOX_NIGHT).expect("embedded Gruvbox theme")
    }
}
