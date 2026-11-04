use super::*;
use crate::frame::{VirtualText, VirtualTextPlacement};
use crate::paint;
use crate::render::highlight::paint::paint_plugin;
use crate::render::virtual_text::{finish_virtual_text, virtual_text_segments};

#[test]
fn virtual_text_sits_after_the_line_and_clips() {
    let text = VirtualText::diagnostic("message", crate::frame::DiagnosticSeverity::Error);
    let segments = virtual_text_segments(10, 30, [&text]);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].column, 11);
    assert_eq!(segments[0].clipped, "message");
    let long = VirtualText::diagnostic("abcdefghijklmnop", crate::frame::DiagnosticSeverity::Error);
    let segments = virtual_text_segments(0, 14, [&long]);
    assert_eq!(segments[0].column, 1);
    assert_eq!(segments[0].clipped, "abcdefghijklm");
    assert!(virtual_text_segments(25, 30, [&text]).is_empty());
}

#[test]
fn virtual_text_uses_display_width_for_tabs_and_wide_characters() {
    let text = VirtualText::diagnostic("中abc", crate::frame::DiagnosticSeverity::Warning);
    let line_width = crate::display::screen_len("a\tb", 8);
    assert_eq!(line_width, 9);
    let segments = virtual_text_segments(line_width, 19, [&text]);
    assert_eq!(segments[0].column, 10);
    assert_eq!(segments[0].clipped, "中abc");
    let wide = VirtualText::diagnostic("中中中中中", crate::frame::DiagnosticSeverity::Warning);
    let segments = virtual_text_segments(line_width, 19, [&wide]);
    assert_eq!(segments[0].column, 10);
    assert_eq!(segments[0].clipped, "中中中中");
}

#[test]
fn plugin_virtual_text_follows_diagnostics() {
    let diagnostic = VirtualText::diagnostic("message", crate::frame::DiagnosticSeverity::Error);
    let plugin = VirtualText::plugin("note", plugin_style(false));
    let segments = virtual_text_segments(0, 40, [&diagnostic, &plugin]);
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[1].column, 9);
    assert_eq!(segments[1].clipped, "note");
}

#[test]
fn metadata_virtual_text_is_placed_before_the_line() {
    let text = VirtualText::metadata("-rw-r--r--    13 2026-10-28 14:32");
    assert_eq!(text.placement, VirtualTextPlacement::BeforeLine);
    assert!(virtual_text_segments(4, 50, [&text]).is_empty());
}

#[test]
fn metadata_suffix_virtual_text_is_placed_after_the_line() {
    let text = VirtualText::metadata_suffix(" -> target.txt");
    assert_eq!(text.placement, VirtualTextPlacement::AfterLine);
    let segments = virtual_text_segments(4, 50, [&text]);
    assert_eq!(segments[0].column, 5);
    assert_eq!(segments[0].clipped, " -> target.txt");
}

#[test]
fn virtual_text_restores_base_colors() {
    let theme = Theme::default();
    let mut out = surface(1, 1, &theme);
    paint::apply_faded_text(&mut out, theme.diagnostic_error);
    finish_virtual_text(&mut out, &theme).unwrap();
    out.write_text("x");
    let style = out.cell(0, 0).expect("restored cell").style();
    assert_eq!(style.foreground(), theme.base.foreground);
    assert_eq!(style.background(), theme.base.background);
    assert!(!style.is_dim());
}

#[test]
fn hidden_plugin_spans_apply_conceal() {
    let style = plugin_style(true);
    let theme = Theme::default();
    let mut out = surface(1, 1, &theme);
    paint_plugin(&mut out, ":", style, &theme, theme.base.background).expect("paint plugin");
    let cell = out.cell(0, 0).expect("plugin cell");
    assert_eq!(cell.text(), ":");
    assert!(cell.style().is_hidden());
}
