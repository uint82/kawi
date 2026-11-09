use kawi_editor::Mode;

pub(super) fn diagnostics_deferred(mode: Mode) -> bool {
    mode.is_insert()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_mode_is_never_deferred() {
        assert!(!diagnostics_deferred(Mode::Normal));
    }

    #[test]
    fn insert_mode_is_deferred() {
        assert!(diagnostics_deferred(Mode::Insert));
    }

    #[test]
    fn replace_mode_is_deferred() {
        assert!(diagnostics_deferred(Mode::Replace));
    }

    #[test]
    fn command_visual_and_search_modes_are_not_deferred() {
        for mode in [
            Mode::Command,
            Mode::Visual,
            Mode::VisualLine,
            Mode::VisualBlock,
            Mode::Search,
        ] {
            assert!(!diagnostics_deferred(mode));
        }
    }
}
