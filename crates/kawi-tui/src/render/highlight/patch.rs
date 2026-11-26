use kawi_syntax::SyntaxHighlight;

use crate::frame::PluginStyle;
use crate::surface::{StylePatch, Surface, UnderlinePatch};
use crate::text::display_width;
use crate::theme::{MatchColors, Theme};

use super::spans::{HighlightSpan, Tint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PatchTarget<'a> {
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) theme: &'a Theme,
}

pub(crate) fn patch_spans_at(out: &mut Surface, spans: &[HighlightSpan], target: PatchTarget<'_>) {
    for span in spans {
        patch_span_at(out, span, target);
    }
}

pub(crate) fn patch_span_at(out: &mut Surface, span: &HighlightSpan, target: PatchTarget<'_>) {
    apply_patch_at(out, span, target, span_patch(span, target.theme));
}

pub(crate) fn apply_patch_at(
    out: &mut Surface,
    span: &HighlightSpan,
    target: PatchTarget<'_>,
    patch: StylePatch,
) {
    out.patch_range(
        target.x.saturating_add(span.from),
        target.y.saturating_add(span.screen_row),
        display_width(&span.text),
        patch,
    );
}

fn span_patch(span: &HighlightSpan, theme: &Theme) -> StylePatch {
    match span.tint {
        Tint::Selection => selection_patch(theme),
        Tint::BracketMatch => bracket_patch(theme),
        Tint::SearchMatch => search_patch(&theme.search_match),
        Tint::SearchCurrent => search_patch(&theme.search_current),
        Tint::Substitute => search_patch(&theme.substitute),
        Tint::Syntax(highlight) => syntax_patch(highlight, theme),
        Tint::Plugin(style) => plugin_patch(style),
        Tint::Diagnostic { .. } => {
            debug_assert!(false, "diagnostic spans use the diagnostic layer");
            StylePatch::default()
        }
    }
}

fn syntax_patch(highlight: SyntaxHighlight, theme: &Theme) -> StylePatch {
    let Some(style) = theme.syntax.style(highlight) else {
        return StylePatch::default();
    };
    StylePatch {
        foreground: style.foreground,
        background: style.background,
        bold: Some(style.bold),
        italic: Some(style.italic),
        underline: Some(underline_patch(style.underline)),
        ..StylePatch::default()
    }
}

pub(crate) fn plugin_patch(style: PluginStyle) -> StylePatch {
    StylePatch {
        foreground: Some(style.foreground),
        background: Some(style.background),
        bold: Some(style.bold),
        dim: Some(false),
        italic: Some(style.italic),
        crossed_out: Some(false),
        hidden: Some(style.hidden),
        underline: Some(underline_patch(style.underline)),
    }
}

fn underline_patch(enabled: bool) -> UnderlinePatch {
    if enabled {
        return UnderlinePatch::Plain;
    }
    UnderlinePatch::Clear
}

fn search_patch(colors: &MatchColors) -> StylePatch {
    StylePatch {
        foreground: Some(colors.foreground),
        background: Some(colors.background),
        ..StylePatch::default()
    }
}

fn selection_patch(theme: &Theme) -> StylePatch {
    StylePatch {
        background: Some(theme.selection.background),
        ..StylePatch::default()
    }
}

fn bracket_patch(theme: &Theme) -> StylePatch {
    StylePatch {
        foreground: Some(theme.bracket_match.foreground),
        background: Some(theme.bracket_match.background),
        bold: Some(true),
        dim: Some(false),
        italic: Some(false),
        crossed_out: Some(false),
        hidden: Some(false),
        underline: Some(UnderlinePatch::Clear),
    }
}
