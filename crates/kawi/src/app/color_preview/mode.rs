#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewMode {
    Stylesheet,
    Markup,
    Plain,
}

impl PreviewMode {
    pub(crate) fn from_language(name: &str) -> Self {
        match name {
            "css" | "scss" | "sass" | "less" => Self::Stylesheet,
            "html" | "vue" | "svelte" | "astro" => Self::Markup,
            _ => Self::Plain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PreviewMode;

    #[test]
    fn maps_stylesheets() {
        for name in ["css", "scss", "sass", "less"] {
            assert_eq!(PreviewMode::from_language(name), PreviewMode::Stylesheet);
        }
    }

    #[test]
    fn maps_markup() {
        for name in ["html", "vue", "svelte", "astro"] {
            assert_eq!(PreviewMode::from_language(name), PreviewMode::Markup);
        }
    }

    #[test]
    fn maps_other_languages_to_plain() {
        assert_eq!(PreviewMode::from_language("rust"), PreviewMode::Plain);
    }
}
