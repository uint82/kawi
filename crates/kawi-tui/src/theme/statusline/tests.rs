use crossterm::style::Color;

use super::*;

#[test]
fn parses_mode_and_diagnostic_overrides() {
    let file: StatusLineFile = toml::from_str(
        r##"
[mode.normal]
foreground = "#010203"
background = "#040506"

[diagnostics.error]
foreground = "#070809"
background = "#0a0b0c"
"##,
    )
    .unwrap();
    let theme = parse(Some(file)).unwrap();
    assert_eq!(
        theme.mode_normal,
        Some(MatchColors {
            foreground: Color::Rgb { r: 1, g: 2, b: 3 },
            background: Color::Rgb { r: 4, g: 5, b: 6 },
        })
    );
    assert_eq!(
        theme.diagnostics_error,
        Some(MatchColors {
            foreground: Color::Rgb { r: 7, g: 8, b: 9 },
            background: Color::Rgb {
                r: 10,
                g: 11,
                b: 12
            },
        })
    );
}

#[test]
fn rejects_unknown_statusline_fields() {
    assert!(toml::from_str::<StatusLineFile>("unknown = 1\n").is_err());
}

#[test]
fn defaults_follow_theme_colors() {
    let theme = Theme::default();
    assert_eq!(
        theme.statusline_colors(SegmentStyle::Default, true),
        theme.status
    );
    assert_eq!(
        theme
            .statusline_colors(SegmentStyle::Mode(ModeKind::Normal), true)
            .background,
        theme.diagnostic_information
    );
    assert_eq!(
        theme
            .statusline_colors(SegmentStyle::Diagnostic(DiagnosticSeverity::Warning), true)
            .background,
        theme.diagnostic_warning
    );
}

#[test]
fn overrides_apply_only_to_active_windows() {
    let file: StatusLineFile = toml::from_str(
        r##"
[mode.insert]
foreground = "#010203"
background = "#040506"
"##,
    )
    .unwrap();
    let theme = Theme {
        statusline: parse(Some(file)).unwrap(),
        ..Theme::default()
    };
    assert_eq!(
        theme
            .statusline_colors(SegmentStyle::Mode(ModeKind::Insert), true)
            .background,
        Color::Rgb { r: 4, g: 5, b: 6 }
    );
    assert_eq!(
        theme
            .statusline_colors(SegmentStyle::Mode(ModeKind::Insert), false)
            .background,
        theme.base.background
    );
}

#[test]
fn theme_files_accept_a_statusline_table() {
    let text = format!(
        "{}\n[statusline.mode.normal]\nforeground = \"#010203\"\nbackground = \"#040506\"\n",
        include_str!("../../../themes/default.toml")
    );
    let theme = Theme::from_toml("default", &text).unwrap();
    assert_eq!(
        theme
            .statusline_colors(SegmentStyle::Mode(ModeKind::Normal), true)
            .background,
        Color::Rgb { r: 4, g: 5, b: 6 }
    );
}
