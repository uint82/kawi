use crossterm::style::Color;

use super::super::{BaseColors, MatchColors, SyntaxStyles, Theme};
use super::palette::{one_dark_palette, rgb};

impl Theme {
    pub(super) fn one_dark() -> Self {
        Self {
            name: "one-dark".to_string(),
            base: BaseColors {
                foreground: rgb(224, 226, 234),
                background: rgb(20, 22, 27),
            },
            float_background: rgb(20, 22, 27),
            code_background: rgb(20, 22, 27),
            search_match: MatchColors {
                foreground: rgb(224, 226, 234),
                background: rgb(107, 83, 0),
            },
            search_current: MatchColors {
                foreground: Color::Black,
                background: rgb(252, 224, 148),
            },
            substitute: MatchColors {
                foreground: rgb(224, 226, 234),
                background: rgb(107, 83, 0),
            },
            selection: MatchColors {
                foreground: Color::Black,
                background: Color::White,
            },
            status: MatchColors {
                foreground: rgb(20, 22, 27),
                background: rgb(224, 226, 234),
            },
            line_number: rgb(79, 82, 88),
            non_text: rgb(79, 82, 88),
            git_add: rgb(152, 195, 121),
            git_change: rgb(229, 192, 123),
            git_delete: rgb(224, 108, 117),
            git_staged_add: rgb(89, 123, 96),
            git_staged_change: rgb(70, 124, 123),
            git_staged_delete: rgb(150, 82, 88),
            diagnostic_error: rgb(224, 108, 117),
            diagnostic_warning: rgb(229, 192, 123),
            diagnostic_information: rgb(97, 175, 239),
            diagnostic_hint: rgb(152, 195, 121),
            syntax: SyntaxStyles::from_keyed(one_dark_palette().styles()),
        }
    }
}
