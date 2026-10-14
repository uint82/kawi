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
    pub base: BaseColors,
    pub search_match: MatchColors,
    pub search_current: MatchColors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_neovim_base() {
        let theme = Theme::default();
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
}
