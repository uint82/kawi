use super::super::{BaseColors, MatchColors, SyntaxStyles, Theme};
use super::palette::{gruvbox_light_palette, rgb};

impl Theme {
    pub(super) fn gruvbox_light() -> Self {
        Self {
            name: "gruvbox-light".to_string(),
            base: BaseColors {
                foreground: rgb(60, 56, 54),
                background: rgb(251, 241, 199),
            },
            float_background: rgb(251, 241, 199),
            code_background: rgb(251, 241, 199),
            search_match: MatchColors {
                foreground: rgb(251, 241, 199),
                background: rgb(181, 118, 20),
            },
            search_current: MatchColors {
                foreground: rgb(251, 241, 199),
                background: rgb(175, 58, 3),
            },
            substitute: MatchColors {
                foreground: rgb(251, 241, 199),
                background: rgb(181, 118, 20),
            },
            selection: MatchColors {
                foreground: rgb(60, 56, 54),
                background: rgb(189, 174, 147),
            },
            status: MatchColors {
                foreground: rgb(60, 56, 54),
                background: rgb(213, 196, 161),
            },
            line_number: rgb(146, 131, 116),
            non_text: rgb(146, 131, 116),
            git_add: rgb(121, 116, 14),
            git_change: rgb(181, 118, 20),
            git_delete: rgb(157, 0, 6),
            git_staged_add: rgb(100, 110, 50),
            git_staged_change: rgb(70, 110, 105),
            git_staged_delete: rgb(135, 70, 70),
            diagnostic_error: rgb(157, 0, 6),
            diagnostic_warning: rgb(181, 118, 20),
            diagnostic_information: rgb(7, 102, 120),
            diagnostic_hint: rgb(121, 116, 14),
            syntax: SyntaxStyles::from_keyed(gruvbox_light_palette().styles()),
        }
    }
}
