use crossterm::style::Color;
use kawi_syntax::SyntaxGroup;

pub struct BaseColors {
    pub foreground: Color,
    pub background: Color,
}

pub struct MatchColors {
    pub foreground: Color,
    pub background: Color,
}

pub struct SyntaxColors {
    comment: Color,
    keyword: Color,
    string: Color,
    number: Color,
    constant: Color,
    function: Color,
    type_name: Color,
    variable: Color,
    property: Color,
    operator: Color,
    punctuation: Color,
    attribute: Color,
    tag: Color,
    invalid: Color,
}

impl SyntaxColors {
    pub fn color(&self, group: SyntaxGroup) -> Color {
        match group {
            SyntaxGroup::Comment => self.comment,
            SyntaxGroup::Keyword => self.keyword,
            SyntaxGroup::String => self.string,
            SyntaxGroup::Number => self.number,
            SyntaxGroup::Constant => self.constant,
            SyntaxGroup::Function => self.function,
            SyntaxGroup::Type => self.type_name,
            SyntaxGroup::Variable => self.variable,
            SyntaxGroup::Property => self.property,
            SyntaxGroup::Operator => self.operator,
            SyntaxGroup::Punctuation => self.punctuation,
            SyntaxGroup::Attribute => self.attribute,
            SyntaxGroup::Tag => self.tag,
            SyntaxGroup::Invalid => self.invalid,
        }
    }

    fn default_palette() -> Self {
        Self {
            comment: rgb(92, 99, 112),
            keyword: rgb(198, 120, 221),
            string: rgb(152, 195, 121),
            number: rgb(209, 154, 102),
            constant: rgb(209, 154, 102),
            function: rgb(97, 175, 239),
            type_name: rgb(229, 192, 123),
            variable: rgb(224, 226, 234),
            property: rgb(86, 182, 194),
            operator: rgb(86, 182, 194),
            punctuation: rgb(171, 178, 191),
            attribute: rgb(229, 192, 123),
            tag: rgb(224, 108, 117),
            invalid: rgb(224, 108, 117),
        }
    }

    fn gruvbox_dark_palette() -> Self {
        Self {
            comment: rgb(146, 131, 116),
            keyword: rgb(251, 73, 52),
            string: rgb(184, 187, 38),
            number: rgb(211, 134, 155),
            constant: rgb(211, 134, 155),
            function: rgb(250, 189, 47),
            type_name: rgb(131, 165, 152),
            variable: rgb(235, 219, 178),
            property: rgb(142, 192, 124),
            operator: rgb(142, 192, 124),
            punctuation: rgb(168, 153, 132),
            attribute: rgb(250, 189, 47),
            tag: rgb(251, 73, 52),
            invalid: rgb(251, 73, 52),
        }
    }

    fn gruvbox_light_palette() -> Self {
        Self {
            comment: rgb(146, 131, 116),
            keyword: rgb(157, 0, 6),
            string: rgb(121, 116, 14),
            number: rgb(143, 63, 113),
            constant: rgb(143, 63, 113),
            function: rgb(181, 118, 20),
            type_name: rgb(7, 102, 120),
            variable: rgb(60, 56, 54),
            property: rgb(66, 123, 88),
            operator: rgb(66, 123, 88),
            punctuation: rgb(124, 111, 100),
            attribute: rgb(181, 118, 20),
            tag: rgb(157, 0, 6),
            invalid: rgb(157, 0, 6),
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

pub struct Theme {
    pub name: &'static str,
    pub base: BaseColors,
    pub search_match: MatchColors,
    pub search_current: MatchColors,
    pub substitute: MatchColors,
    pub selection: MatchColors,
    pub status: MatchColors,
    pub line_number: Color,
    pub non_text: Color,
    pub syntax: SyntaxColors,
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
            substitute: MatchColors {
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
            selection: MatchColors {
                foreground: Color::Black,
                background: Color::White,
            },
            status: MatchColors {
                foreground: Color::Rgb {
                    r: 20,
                    g: 22,
                    b: 27,
                },
                background: Color::Rgb {
                    r: 224,
                    g: 226,
                    b: 234,
                },
            },
            line_number: Color::Rgb {
                r: 79,
                g: 82,
                b: 88,
            },
            non_text: Color::Rgb {
                r: 79,
                g: 82,
                b: 88,
            },
            syntax: SyntaxColors::default_palette(),
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
            substitute: MatchColors {
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
            selection: MatchColors {
                foreground: Color::Rgb {
                    r: 235,
                    g: 219,
                    b: 178,
                },
                background: Color::Rgb {
                    r: 102,
                    g: 92,
                    b: 84,
                },
            },
            status: MatchColors {
                foreground: Color::Rgb {
                    r: 235,
                    g: 219,
                    b: 178,
                },
                background: Color::Rgb {
                    r: 80,
                    g: 73,
                    b: 69,
                },
            },
            line_number: Color::Rgb {
                r: 146,
                g: 131,
                b: 116,
            },
            non_text: Color::Rgb {
                r: 146,
                g: 131,
                b: 116,
            },
            syntax: SyntaxColors::gruvbox_dark_palette(),
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
            substitute: MatchColors {
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
            selection: MatchColors {
                foreground: Color::Rgb {
                    r: 60,
                    g: 56,
                    b: 54,
                },
                background: Color::Rgb {
                    r: 189,
                    g: 174,
                    b: 147,
                },
            },
            status: MatchColors {
                foreground: Color::Rgb {
                    r: 60,
                    g: 56,
                    b: 54,
                },
                background: Color::Rgb {
                    r: 213,
                    g: 196,
                    b: 161,
                },
            },
            line_number: Color::Rgb {
                r: 146,
                g: 131,
                b: 116,
            },
            non_text: Color::Rgb {
                r: 146,
                g: 131,
                b: 116,
            },
            syntax: SyntaxColors::gruvbox_light_palette(),
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
    fn substitute_matches_search_by_default() {
        for name in Theme::names() {
            let theme = Theme::named(name).expect("theme");
            assert_eq!(theme.substitute.foreground, theme.search_match.foreground);
            assert_eq!(theme.substitute.background, theme.search_match.background);
        }
    }

    #[test]
    fn selection_uses_light_background() {
        let theme = Theme::default();
        assert_eq!(theme.selection.foreground, Color::Black);
        assert_eq!(theme.selection.background, Color::White);
    }

    #[test]
    fn status_reverses_base() {
        let theme = Theme::default();
        assert_eq!(theme.status.foreground, theme.base.background);
        assert_eq!(theme.status.background, theme.base.foreground);
    }

    #[test]
    fn gruvbox_dark_selection_and_status() {
        let theme = Theme::named("gruvbox-dark").expect("theme");
        assert_eq!(
            theme.selection.background,
            Color::Rgb {
                r: 102,
                g: 92,
                b: 84
            }
        );
        assert_eq!(
            theme.status.background,
            Color::Rgb {
                r: 80,
                g: 73,
                b: 69
            }
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

#[cfg(test)]
mod syntax_tests {
    use super::*;

    #[test]
    fn every_theme_colors_every_group() {
        for name in Theme::names() {
            let theme = Theme::named(name).expect("theme");
            for group in SyntaxGroup::ALL {
                assert_ne!(theme.syntax.color(group), theme.base.background);
            }
        }
    }

    #[test]
    fn default_syntax_uses_one_dark_accents() {
        let theme = Theme::default();
        assert_eq!(theme.syntax.color(SyntaxGroup::Keyword), rgb(198, 120, 221));
        assert_eq!(theme.syntax.color(SyntaxGroup::String), rgb(152, 195, 121));
        assert_eq!(theme.syntax.color(SyntaxGroup::Comment), rgb(92, 99, 112));
    }
}
