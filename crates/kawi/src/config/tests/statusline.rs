use super::*;

    #[test]
    fn loads_statusline_elements_and_mode_labels() {
        let loaded = load_text(
            r#"
[ui.statusline]
left = ["mode", "file", "diagnostics"]
center = ["position"]
right = ["filetype", "position"]
separator = " | "
color_modes = false

[ui.statusline.mode]
normal = "N"
insert = "I"
"#,
        );
        assert!(loaded.warnings.is_empty());
        let statusline = loaded.config.statusline();
        assert_eq!(
            statusline.left,
            vec![
                kawi_tui::statusbar::SegmentId::Mode,
                kawi_tui::statusbar::SegmentId::File,
                kawi_tui::statusbar::SegmentId::Diagnostics,
            ]
        );
        assert_eq!(
            statusline.center,
            vec![kawi_tui::statusbar::SegmentId::Position]
        );
        assert_eq!(
            statusline.right,
            vec![
                kawi_tui::statusbar::SegmentId::FileType,
                kawi_tui::statusbar::SegmentId::Position,
            ]
        );
        assert_eq!(statusline.separator, " | ");
        assert!(!statusline.color_modes);
        assert_eq!(
            statusline
                .mode_labels
                .get(&kawi_tui::statusbar::ModeKind::Normal),
            Some(&"N".to_string())
        );
    }

    #[test]
    fn rejects_unknown_statusline_elements_and_labels() {
        let loaded = load_text(
            r#"
[ui.statusline]
left = ["mode", "nope", 3]
color_modes = "yes"

[ui.statusline.mode]
normal = "N"
wat = "W"
"#,
        );
        assert_eq!(
            loaded.config.statusline().left,
            vec![kawi_tui::statusbar::SegmentId::Mode]
        );
        assert!(loaded.config.statusline().color_modes);
        assert_eq!(loaded.warnings.len(), 4);
    }
