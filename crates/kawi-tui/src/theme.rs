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
                background: Color::Yellow,
            },
            search_current: MatchColors {
                foreground: Color::Black,
                background: Color::DarkYellow,
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
        assert_eq!(theme.search_match.background, Color::Yellow);
        assert_eq!(theme.search_current.foreground, Color::Black);
        assert_eq!(theme.search_current.background, Color::DarkYellow);
    }

    #[test]
    fn current_match_is_darker() {
        let theme = Theme::default();
        assert_ne!(
            theme.search_match.background,
            theme.search_current.background
        );
    }
}
