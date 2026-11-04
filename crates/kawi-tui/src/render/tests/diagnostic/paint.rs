use super::*;

#[test]
fn faded_diagnostics_keep_the_theme_background() {
    let theme = Theme::default();
    let severity = crate::frame::DiagnosticSeverity::Hint;
    let mut out = surface(1, 1, &theme);
    paint_diagnostic(
        &mut out,
        "x",
        DiagnosticPaint {
            severity,
            tag: Some(crate::frame::DiagnosticTag::Unnecessary),
            selected: false,
            syntax: None,
            underline: DiagnosticUnderline::default(),
        },
        &theme,
    )
    .unwrap();
    let cell = out.cell(0, 0).expect("diagnostic cell");
    assert_eq!(cell.text(), "x");
    assert_eq!(cell.style().foreground(), theme.diagnostic_color(severity));
    assert_eq!(cell.style().background(), theme.base.background);
    assert!(cell.style().is_dim());
}

#[test]
fn selected_diagnostics_keep_selection_background_and_underline() {
    let theme = Theme::default();
    let mut out = surface(1, 1, &theme);
    paint_diagnostic(
        &mut out,
        "x",
        DiagnosticPaint {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
            selected: true,
            syntax: None,
            underline: DiagnosticUnderline::default(),
        },
        &theme,
    )
    .unwrap();
    let cell = out.cell(0, 0).expect("diagnostic cell");
    assert_eq!(cell.text(), "x");
    assert_eq!(cell.style().foreground(), theme.base.foreground);
    assert_eq!(cell.style().background(), theme.selection.background);
    assert_eq!(cell.style().underline(), Some(DiagnosticUnderline::Curly));
    assert_eq!(
        cell.style().underline_color(),
        Some(theme.diagnostic_color(crate::frame::DiagnosticSeverity::Error))
    );
}

#[test]
fn diagnostic_underline_style_selects_the_terminal_attribute() {
    let theme = Theme::default();
    let paint = |underline| {
        let mut out = surface(1, 1, &theme);
        paint_diagnostic(
            &mut out,
            "x",
            DiagnosticPaint {
                severity: crate::frame::DiagnosticSeverity::Error,
                tag: None,
                selected: false,
                syntax: None,
                underline,
            },
            &theme,
        )
        .unwrap();
        out.cell(0, 0).expect("diagnostic cell").style().underline()
    };
    assert_eq!(
        paint(DiagnosticUnderline::Curly),
        Some(DiagnosticUnderline::Curly)
    );
    assert_eq!(
        paint(DiagnosticUnderline::Straight),
        Some(DiagnosticUnderline::Straight)
    );
}

#[test]
fn selected_diagnostics_keep_syntax_and_selection_background() {
    let theme = Theme::default();
    let keyword = highlight_named("keyword");
    let mut out = surface(2, 1, &theme);
    paint_diagnostic(
        &mut out,
        "fn",
        DiagnosticPaint {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
            selected: true,
            syntax: Some(keyword),
            underline: DiagnosticUnderline::default(),
        },
        &theme,
    )
    .unwrap();
    let style = out.cell(0, 0).expect("diagnostic cell").style();
    assert_eq!(out.cell(0, 0).expect("diagnostic cell").text(), "f");
    assert_eq!(out.cell(1, 0).expect("diagnostic cell").text(), "n");
    assert_eq!(
        style.foreground(),
        theme
            .syntax
            .named("keyword")
            .expect("keyword")
            .foreground
            .expect("foreground")
    );
    assert_eq!(style.background(), theme.selection.background);
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}

#[test]
fn unselected_diagnostics_keep_syntax_foreground() {
    let theme = Theme::default();
    let keyword = highlight_named("keyword");
    let mut out = surface(2, 1, &theme);
    paint_diagnostic(
        &mut out,
        "fn",
        DiagnosticPaint {
            severity: crate::frame::DiagnosticSeverity::Error,
            tag: None,
            selected: false,
            syntax: Some(keyword),
            underline: DiagnosticUnderline::default(),
        },
        &theme,
    )
    .unwrap();
    let style = out.cell(0, 0).expect("diagnostic cell").style();
    assert_eq!(
        style.foreground(),
        theme
            .syntax
            .named("keyword")
            .expect("keyword")
            .foreground
            .expect("foreground")
    );
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
    assert_eq!(out.cell(1, 0).expect("diagnostic cell").text(), "n");
}

#[test]
fn tagged_diagnostics_keep_syntax_foreground() {
    let theme = Theme::default();
    let keyword = highlight_named("keyword");
    let expected = theme
        .syntax
        .named("keyword")
        .and_then(|style| style.foreground)
        .expect("keyword foreground");
    for (tag, dim, crossed) in [
        (crate::frame::DiagnosticTag::Unnecessary, true, false),
        (crate::frame::DiagnosticTag::Deprecated, false, true),
    ] {
        let mut out = surface(2, 1, &theme);
        paint_diagnostic(
            &mut out,
            "fn",
            DiagnosticPaint {
                severity: crate::frame::DiagnosticSeverity::Warning,
                tag: Some(tag),
                selected: false,
                syntax: Some(keyword),
                underline: DiagnosticUnderline::default(),
            },
            &theme,
        )
        .unwrap();
        let style = out.cell(0, 0).expect("diagnostic cell").style();
        assert_eq!(style.foreground(), expected);
        assert_eq!(style.is_dim(), dim);
        assert_eq!(style.is_crossed_out(), crossed);
        assert_eq!(out.cell(1, 0).expect("diagnostic cell").text(), "n");
    }
}
