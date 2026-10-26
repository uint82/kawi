use super::Theme;

mod gruvbox_light;
mod one_dark;
mod palette;

const GRUVBOX_NIGHT: &str = include_str!("../../themes/gruvbox-night.toml");

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

    pub fn names() -> [&'static str; 3] {
        ["default", "one-dark", "gruvbox-light"]
    }

    pub fn default_theme() -> Self {
        Self::gruvbox_night("default")
    }

    fn gruvbox_night(name: &str) -> Self {
        Self::from_toml(name, GRUVBOX_NIGHT).expect("embedded Gruvbox theme")
    }
}
