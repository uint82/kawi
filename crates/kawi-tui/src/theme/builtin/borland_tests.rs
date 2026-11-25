use std::collections::BTreeSet;

use kawi_completion::CompletionKind;

use super::test_support::{contrast_ratio, leaf_keys, rgb, table_keys};
use super::*;

const DECORATION_STYLES: &[&str] = &["accent", "info", "success", "warning", "error", "muted"];

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

#[test]
fn borland_defines_every_registry_capture() {
    let expected: BTreeSet<String> = kawi_syntax::registry().names().iter().cloned().collect();
    assert_eq!(
        super::test_support::syntax_keys(BORLAND),
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
    for key in super::test_support::syntax_keys(BORLAND) {
        let foreground = theme
            .syntax
            .named(&key)
            .and_then(|style| style.foreground)
            .expect("syntax color");
        assert!(
            contrast_ratio(foreground, theme.base.background) >= 4.5,
            "{key} is below 4.5:1 contrast"
        );
    }
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
        assert!(contrast_ratio(foreground, background) >= 4.5);
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
