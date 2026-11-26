use std::collections::BTreeSet;

use crossterm::style::Color;
use kawi_completion::CompletionKind;

use super::test_support::{
    contrast_ratio, leaf_keys, rgb, syntax_keys, table_keys, MIN_BODY_CONTRAST,
};
use super::*;

const DECORATION_STYLES: &[&str] = &["accent", "info", "success", "warning", "error", "muted"];

const MIN_SELECTION_TINT: f64 = 1.3;

const STATUSLINE_SEGMENTS: &[&str] = &[
    "mode.normal",
    "mode.insert",
    "mode.replace",
    "mode.visual",
    "mode.command",
    "mode.search",
    "diagnostics.error",
    "diagnostics.warning",
    "diagnostics.information",
    "diagnostics.hint",
    "modified",
    "readonly",
    "git",
    "lsp",
    "selection",
    "macro_recording",
    "filetype",
    "indent",
    "encoding",
    "eol",
    "percent",
    "position",
];

fn keys(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn completion_keys() -> BTreeSet<String> {
    CompletionKind::ALL
        .iter()
        .map(|kind| kind.key().to_string())
        .chain(["match".to_string()])
        .collect()
}

fn statusline_keys() -> BTreeSet<String> {
    STATUSLINE_SEGMENTS
        .iter()
        .flat_map(|name| [format!("{name}.foreground"), format!("{name}.background")])
        .collect()
}

fn assert_syntax_contrast(theme: &Theme, background: Color) {
    for key in syntax_keys(BORLAND) {
        let foreground = theme
            .syntax
            .named(&key)
            .and_then(|style| style.foreground)
            .expect("syntax color");
        assert!(
            contrast_ratio(foreground, background) >= MIN_BODY_CONTRAST,
            "{key} is below {MIN_BODY_CONTRAST}:1 contrast"
        );
    }
}

#[test]
fn borland_defines_every_registry_capture() {
    let expected: BTreeSet<String> = kawi_syntax::registry().names().iter().cloned().collect();
    assert_eq!(
        syntax_keys(BORLAND),
        expected,
        "borland syntax must define every registered capture"
    );
}

#[test]
fn borland_covers_every_ui_section() {
    assert_eq!(table_keys(BORLAND, "completion"), completion_keys());
    assert_eq!(table_keys(BORLAND, "decorations"), keys(DECORATION_STYLES));
    assert_eq!(leaf_keys(BORLAND, "statusline"), statusline_keys());
}

#[test]
fn borland_syntax_meets_body_contrast() {
    let theme = Theme::named("borland").expect("theme");
    assert_syntax_contrast(&theme, theme.base.background);
}

#[test]
fn borland_selection_keeps_syntax_readable() {
    let theme = Theme::named("borland").expect("theme");
    assert_syntax_contrast(&theme, theme.selection.background);
    assert!(
        contrast_ratio(theme.selection.foreground, theme.selection.background) >= MIN_BODY_CONTRAST,
        "selection foreground is below {MIN_BODY_CONTRAST}:1 contrast"
    );
}

#[test]
fn borland_selection_stands_out_from_the_editor_background() {
    let theme = Theme::named("borland").expect("theme");
    let tint = contrast_ratio(theme.selection.background, theme.base.background);
    assert!(
        tint >= MIN_SELECTION_TINT,
        "selection background is only {tint:.2}:1 from the editor background"
    );
}

#[test]
fn borland_ui_meets_contrast() {
    let theme = Theme::named("borland").expect("theme");
    for (foreground, background) in [
        (theme.base.foreground, theme.base.background),
        (theme.selection.foreground, theme.selection.background),
        (theme.status.foreground, theme.status.background),
        (theme.search_match.foreground, theme.search_match.background),
        (
            theme.search_current.foreground,
            theme.search_current.background,
        ),
        (theme.substitute.foreground, theme.substitute.background),
        (
            theme.bracket_match.foreground,
            theme.bracket_match.background,
        ),
        (theme.line_number, theme.base.background),
        (theme.non_text, theme.base.background),
        (theme.special_key, theme.base.background),
    ] {
        assert!(contrast_ratio(foreground, background) >= MIN_BODY_CONTRAST);
    }
}

#[test]
fn borland_statusline_overrides_stay_visible_on_the_status_bar() {
    let theme = Theme::named("borland").expect("theme");
    for (name, colors) in [
        ("mode.visual", &theme.statusline.mode_visual),
        ("readonly", &theme.statusline.readonly),
        ("selection", &theme.statusline.selection),
    ] {
        let colors = colors.as_ref().expect("status line color");
        assert_ne!(
            colors.background, theme.status.background,
            "{name} blends into the status bar"
        );
        assert!(
            contrast_ratio(colors.foreground, colors.background) >= MIN_BODY_CONTRAST,
            "{name} is below {MIN_BODY_CONTRAST}:1 contrast"
        );
    }
}

#[test]
fn borland_subcaptures_keep_borland_accents() {
    let theme = Theme::named("borland").expect("theme");
    assert_eq!(
        theme
            .syntax
            .named("keyword.directive")
            .and_then(|style| style.foreground),
        Some(rgb(87, 255, 87))
    );
    assert!(theme.syntax.named("comment.documentation").unwrap().bold);
    assert!(theme.syntax.named("markup.strong").unwrap().bold);
    assert!(theme.syntax.named("markup.underline").unwrap().underline);
}
