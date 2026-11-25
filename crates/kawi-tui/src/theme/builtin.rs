use super::Theme;

const CLASSIC: &str = include_str!("../../themes/classic.toml");
const COBALT: &str = include_str!("../../themes/cobalt.toml");
const DEFAULT_THEME: &str = include_str!("../../themes/default.toml");
const DEFAULT_THEME_NAME: &str = "default";
const GRUVBOX_DARK: &str = include_str!("../../themes/gruvbox-dark.toml");
const GRUVBOX_LIGHT: &str = include_str!("../../themes/gruvbox-light.toml");
const ONE_DARK: &str = include_str!("../../themes/one-dark.toml");

struct BuiltinTheme {
    name: &'static str,
    source: &'static str,
}

const BUILTIN_THEMES: &[BuiltinTheme] = &[
    BuiltinTheme {
        name: DEFAULT_THEME_NAME,
        source: DEFAULT_THEME,
    },
    BuiltinTheme {
        name: "classic",
        source: CLASSIC,
    },
    BuiltinTheme {
        name: "cobalt",
        source: COBALT,
    },
    BuiltinTheme {
        name: "one-dark",
        source: ONE_DARK,
    },
    BuiltinTheme {
        name: "gruvbox-dark",
        source: GRUVBOX_DARK,
    },
    BuiltinTheme {
        name: "gruvbox-light",
        source: GRUVBOX_LIGHT,
    },
];

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
        BUILTIN_THEMES
            .iter()
            .find(|theme| theme.name == name)
            .map(|theme| Self::embedded(theme.name, theme.source))
    }

    pub fn names() -> [&'static str; BUILTIN_THEMES.len()] {
        let mut names = [""; BUILTIN_THEMES.len()];
        for (index, theme) in BUILTIN_THEMES.iter().enumerate() {
            names[index] = theme.name;
        }
        names
    }

    pub fn default_theme() -> Self {
        Self::named(DEFAULT_THEME_NAME).expect("embedded default theme")
    }

    fn embedded(name: &str, source: &str) -> Self {
        Self::from_toml(name, source).expect("embedded theme")
    }
}
