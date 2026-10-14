use crossterm::style::Color;

pub struct MatchColors {
    pub foreground: Color,
    pub background: Color,
}

pub struct Theme {
    pub search_match: MatchColors,
    pub search_current: MatchColors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            search_match: MatchColors {
                foreground: Color::Black,
                background: Color::DarkYellow,
            },
            search_current: MatchColors {
                foreground: Color::Black,
                background: Color::Yellow,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_neovim_palette() {
        let theme = Theme::default();
        assert_eq!(theme.search_match.foreground, Color::Black);
        assert_eq!(theme.search_match.background, Color::DarkYellow);
        assert_eq!(theme.search_current.foreground, Color::Black);
        assert_eq!(theme.search_current.background, Color::Yellow);
    }
}
