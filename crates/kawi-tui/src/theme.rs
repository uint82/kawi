use crossterm::style::Color;

pub struct BaseColors {
    pub foreground: Color,
    pub background: Color,
}

pub struct MatchColors {
    pub foreground: Color,
    pub background: Color,
}

pub struct Theme {
    pub name: &'static str,
    pub base: BaseColors,
    pub search_match: MatchColors,
    pub search_current: MatchColors,
}

impl Theme {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default_theme()),
            "gruvbox-dark" => Some(Self::gruvbox_dark()),
            "gruvbox-light" => Some(Self::gruvbox_light()),
            _ => None,
        }
    }

    pub fn names() -> [&'static str; 3] {
        ["default", "gruvbox-dark", "gruvbox-light"]
    }

    pub fn default_theme() -> Self {
        Self {
            name: "default",
            base: BaseColors {
                foreground: Color::Rgb {
                    r: 224,
                    g: 226,
                    b: 234,
                },
                background: Color::Rgb {
                    r: 20,
                    g: 22,
                    b: 27,
                },
            },
            search_match: MatchColors {
                foreground: Color::Rgb {
                    r: 224,
                    g: 226,
                    b: 234,
                },
                background: Color::Rgb {
                    r: 107,
                    g: 83,
                    b: 0,
                },
            },
            search_current: MatchColors {
                foreground: Color::Black,
                background: Color::Rgb {
                    r: 252,
                    g: 224,
                    b: 148,
                },
            },
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            name: "gruvbox-dark",
            base: BaseColors {
                foreground: Color::Rgb {
                    r: 235,
                    g: 219,
                    b: 178,
                },
                background: Color::Rgb {
                    r: 40,
                    g: 40,
                    b: 40,
                },
            },
            search_match: MatchColors {
                foreground: Color::Rgb {
                    r: 40,
                    g: 40,
                    b: 40,
                },
                background: Color::Rgb {
                    r: 250,
                    g: 189,
                    b: 47,
                },
            },
            search_current: MatchColors {
                foreground: Color::Rgb {
                    r: 40,
                    g: 40,
                    b: 40,
                },
                background: Color::Rgb {
                    r: 254,
                    g: 128,
                    b: 25,
                },
            },
        }
    }

    pub fn gruvbox_light() -> Self {
        Self {
            name: "gruvbox-light",
            base: BaseColors {
                foreground: Color::Rgb {
                    r: 60,
                    g: 56,
                    b: 54,
                },
                background: Color::Rgb {
                    r: 251,
                    g: 241,
                    b: 199,
                },
            },
            search_match: MatchColors {
                foreground: Color::Rgb {
                    r: 251,
                    g: 241,
                    b: 199,
                },
                background: Color::Rgb {
                    r: 181,
                    g: 118,
                    b: 20,
                },
            },
            search_current: MatchColors {
                foreground: Color::Rgb {
                    r: 251,
                    g: 241,
                    b: 199,
                },
                background: Color::Rgb {
                    r: 175,
                    g: 58,
                    b: 3,
                },
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_resolves_all_names() {
        for name in Theme::names() {
            assert_eq!(Theme::named(name).map(|theme| theme.name), Some(name));
        }
        assert!(Theme::named("nope").is_none());
    }

    #[test]
    fn defaults_follow_neovim_base() {
        let theme = Theme::default();
        assert_eq!(theme.name, "default");
        assert_eq!(
            theme.base.background,
            Color::Rgb {
                r: 20,
                g: 22,
                b: 27
            }
        );
        assert_eq!(
            theme.base.foreground,
            Color::Rgb {
                r: 224,
                g: 226,
                b: 234
            }
        );
    }

    #[test]
    fn defaults_follow_neovim_palette() {
        let theme = Theme::default();
        assert_eq!(
            theme.search_match.foreground,
            Color::Rgb {
                r: 224,
                g: 226,
                b: 234
            }
        );
        assert_eq!(
            theme.search_match.background,
            Color::Rgb {
                r: 107,
                g: 83,
                b: 0
            }
        );
        assert_eq!(theme.search_current.foreground, Color::Black);
        assert_eq!(
            theme.search_current.background,
            Color::Rgb {
                r: 252,
                g: 224,
                b: 148
            }
        );
    }

    #[test]
    fn current_match_differs_from_background() {
        let theme = Theme::default();
        assert_ne!(
            theme.search_match.background,
            theme.search_current.background
        );
    }

    #[test]
    fn gruvbox_dark_matches_user_neovim() {
        let theme = Theme::named("gruvbox-dark").expect("theme");
        assert_eq!(
            theme.base.background,
            Color::Rgb {
                r: 40,
                g: 40,
                b: 40
            }
        );
        assert_eq!(
            theme.base.foreground,
            Color::Rgb {
                r: 235,
                g: 219,
                b: 178
            }
        );
        assert_eq!(
            theme.search_match.background,
            Color::Rgb {
                r: 250,
                g: 189,
                b: 47
            }
        );
        assert_eq!(
            theme.search_current.background,
            Color::Rgb {
                r: 254,
                g: 128,
                b: 25
            }
        );
    }

    #[test]
    fn gruvbox_light_uses_dark_accents() {
        let theme = Theme::named("gruvbox-light").expect("theme");
        assert_eq!(
            theme.base.background,
            Color::Rgb {
                r: 251,
                g: 241,
                b: 199
            }
        );
        assert_eq!(
            theme.search_match.background,
            Color::Rgb {
                r: 181,
                g: 118,
                b: 20
            }
        );
        assert_eq!(
            theme.search_current.background,
            Color::Rgb {
                r: 175,
                g: 58,
                b: 3
            }
        );
    }
}
