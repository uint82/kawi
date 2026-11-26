use super::*;
use crate::render::highlight::patch::{patch_span_at, PatchTarget};

fn diagnostic(
    from: usize,
    text: &str,
    severity: DiagnosticSeverity,
    tag: Option<DiagnosticTag>,
) -> HighlightSpan {
    HighlightSpan {
        screen_row: 0,
        from,
        text: text.to_string(),
        tint: Tint::Diagnostic { severity, tag },
    }
}

fn diagnostic_layer<'a>(
    spans: &'a [HighlightSpan],
    theme: &'a Theme,
    underline: DiagnosticUnderline,
) -> DiagnosticLayer<'a> {
    DiagnosticLayer {
        spans,
        target: PatchTarget { x: 0, y: 0, theme },
        underline,
    }
}

fn paint(out: &mut Surface, text: &str, tint: Tint, theme: &Theme) {
    let span = HighlightSpan {
        screen_row: 0,
        from: 0,
        text: text.to_string(),
        tint,
    };
    patch_span_at(out, &span, PatchTarget { x: 0, y: 0, theme });
}

fn apply_diagnostic(
    out: &mut Surface,
    spans: &[HighlightSpan],
    theme: &Theme,
    underline: DiagnosticUnderline,
) {
    patch_decorations(out, &diagnostic_layer(spans, theme, underline));
}

fn style_at(out: &Surface, x: usize) -> crate::surface::Style {
    out.cell(x, 0).expect("cell").style()
}

#[test]
fn search_highlight_survives_a_diagnostic_underline() {
    let theme = Theme::default();
    let mut out = surface(2, 1, &theme);
    out.write_text("fn");
    paint(&mut out, "fn", Tint::SearchMatch, &theme);
    let spans = [diagnostic(0, "fn", DiagnosticSeverity::Error, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    let style = style_at(&out, 0);
    assert_eq!(out.cell(0, 0).expect("cell").text(), "f");
    assert_eq!(style.foreground(), theme.search_match.foreground);
    assert_eq!(style.background(), theme.search_match.background);
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
    assert_eq!(
        style.underline_color(),
        Some(theme.diagnostic_color(DiagnosticSeverity::Error))
    );
}

#[test]
fn current_search_highlight_survives_a_diagnostic_underline() {
    let theme = Theme::default();
    let mut out = surface(2, 1, &theme);
    out.write_text("fn");
    paint(&mut out, "fn", Tint::SearchCurrent, &theme);
    let spans = [diagnostic(0, "fn", DiagnosticSeverity::Warning, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    let style = style_at(&out, 0);
    assert_eq!(style.foreground(), theme.search_current.foreground);
    assert_eq!(style.background(), theme.search_current.background);
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}

#[test]
fn selection_survives_a_diagnostic_underline() {
    let theme = Theme::default();
    let mut out = surface(2, 1, &theme);
    out.write_text("fn");
    paint(&mut out, "fn", Tint::Selection, &theme);
    let spans = [diagnostic(0, "fn", DiagnosticSeverity::Error, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    let style = style_at(&out, 0);
    assert_eq!(style.foreground(), theme.base.foreground);
    assert_eq!(style.background(), theme.selection.background);
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}

#[test]
fn syntax_foreground_survives_a_diagnostic_underline() {
    let theme = Theme::default();
    let keyword = highlight_named("keyword");
    let mut out = surface(2, 1, &theme);
    out.write_text("fn");
    paint(&mut out, "fn", Tint::Syntax(keyword), &theme);
    let spans = [diagnostic(0, "fn", DiagnosticSeverity::Error, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    let style = style_at(&out, 0);
    let expected = theme
        .syntax
        .named("keyword")
        .and_then(|style| style.foreground)
        .expect("keyword foreground");
    assert_eq!(style.foreground(), expected);
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}

#[test]
fn plugin_decoration_survives_a_diagnostic_underline() {
    let theme = Theme::default();
    let mut plugin = plugin_style(false);
    plugin.bold = true;
    plugin.italic = true;
    let mut out = surface(1, 1, &theme);
    out.write_text("x");
    paint(&mut out, "x", Tint::Plugin(plugin), &theme);
    let spans = [diagnostic(0, "x", DiagnosticSeverity::Error, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    let style = style_at(&out, 0);
    assert_eq!(style.foreground(), plugin.foreground);
    assert_eq!(style.background(), plugin.background);
    assert!(style.is_bold());
    assert!(style.is_italic());
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}

#[test]
fn bracket_match_survives_a_diagnostic_underline() {
    let theme = Theme::default();
    let mut out = surface(1, 1, &theme);
    out.write_text("(");
    paint(&mut out, "(", Tint::BracketMatch, &theme);
    let spans = [diagnostic(0, "(", DiagnosticSeverity::Error, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    let style = style_at(&out, 0);
    assert_eq!(style.foreground(), theme.bracket_match.foreground);
    assert_eq!(style.background(), theme.bracket_match.background);
    assert!(style.is_bold());
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}

#[test]
fn substitute_highlight_survives_a_diagnostic_underline() {
    let theme = Theme::default();
    let mut out = surface(3, 1, &theme);
    out.write_text("old");
    paint(&mut out, "old", Tint::Substitute, &theme);
    let spans = [diagnostic(0, "old", DiagnosticSeverity::Hint, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    let style = style_at(&out, 0);
    assert_eq!(style.foreground(), theme.substitute.foreground);
    assert_eq!(style.background(), theme.substitute.background);
    assert_eq!(style.underline(), Some(DiagnosticUnderline::Curly));
}

#[test]
fn tagged_diagnostics_keep_the_tag_foreground_without_an_underline() {
    let theme = Theme::default();
    let severity = DiagnosticSeverity::Warning;
    for (tag, is_dim, is_crossed) in [
        (DiagnosticTag::Unnecessary, true, false),
        (DiagnosticTag::Deprecated, false, true),
    ] {
        let mut out = surface(1, 1, &theme);
        out.write_text("x");
        let spans = [diagnostic(0, "x", severity, Some(tag))];
        let layer = diagnostic_layer(&spans, &theme, DiagnosticUnderline::Curly);
        patch_tag_foreground(&mut out, &layer);
        patch_decorations(&mut out, &layer);
        let style = style_at(&out, 0);
        assert_eq!(style.foreground(), theme.diagnostic_color(severity));
        assert_eq!(style.is_dim(), is_dim);
        assert_eq!(style.is_crossed_out(), is_crossed);
        assert_eq!(style.underline(), None);
    }
}

#[test]
fn tagged_diagnostic_foreground_is_lower_priority_than_syntax() {
    let theme = Theme::default();
    let keyword = highlight_named("keyword");
    let mut out = surface(2, 1, &theme);
    out.write_text("fn");
    let spans = [diagnostic(
        0,
        "fn",
        DiagnosticSeverity::Warning,
        Some(DiagnosticTag::Unnecessary),
    )];
    let layer = diagnostic_layer(&spans, &theme, DiagnosticUnderline::Curly);
    patch_tag_foreground(&mut out, &layer);
    paint(&mut out, "fn", Tint::Syntax(keyword), &theme);
    patch_decorations(&mut out, &layer);
    let style = style_at(&out, 0);
    let expected = theme
        .syntax
        .named("keyword")
        .and_then(|style| style.foreground)
        .expect("keyword foreground");
    assert_eq!(style.foreground(), expected);
    assert!(style.is_dim());
}

#[test]
fn diagnostic_underline_keeps_wide_glyphs_and_tabs_intact() {
    let theme = Theme::default();
    let lines = lines_of("\t\u{4e2d}");
    let viewport = viewport();
    let view = span_view(&lines, &viewport);
    let diagnostic = DiagnosticSpan {
        row: 0,
        start: 0,
        end: 2,
        severity: DiagnosticSeverity::Error,
        tag: None,
    };
    let spans = diagnostic_highlight_spans(&[diagnostic], &view);
    let mut out = surface(10, 1, &theme);
    out.write_text("\t\u{4e2d}");
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Curly);
    assert_eq!(out.cell(0, 0).expect("tab cell").text(), " ");
    assert_eq!(out.cell(8, 0).expect("wide cell").text(), "\u{4e2d}");
    assert!(out.cell(9, 0).expect("continuation").is_continuation());
    assert_eq!(
        style_at(&out, 8).underline(),
        Some(DiagnosticUnderline::Curly)
    );
    assert_eq!(
        style_at(&out, 9).underline(),
        Some(DiagnosticUnderline::Curly)
    );
}

#[test]
fn diagnostic_underline_uses_the_configured_style() {
    let theme = Theme::default();
    let mut out = surface(1, 1, &theme);
    out.write_text("x");
    let spans = [diagnostic(0, "x", DiagnosticSeverity::Error, None)];
    apply_diagnostic(&mut out, &spans, &theme, DiagnosticUnderline::Straight);
    assert_eq!(
        style_at(&out, 0).underline(),
        Some(DiagnosticUnderline::Straight)
    );
}
