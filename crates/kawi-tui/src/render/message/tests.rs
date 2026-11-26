use super::*;
use crate::frame::CursorStyle;
use crate::statusbar::StatusBarConfig;
use crate::theme::Theme;

fn notice_frame<'a>(
    message: &'a str,
    severity: MessageSeverity,
    viewport: &'a Viewport,
    theme: &'a Theme,
    statusline: &'a StatusBarConfig,
) -> Frame<'a> {
    Frame {
        panes: &[],
        separators: &[],
        pager: None,
        completion: None,
        overlay: None,
        picker: None,
        replace: None,
        explorer: None,
        debug: None,
        task: None,
        plugin_panel: None,
        terminal: None,
        viewport,
        message: Some(message),
        message_severity: severity,
        prompt: None,
        command: None,
        search: None,
        search_delimiter: '/',
        search_status: None,
        showcmd: None,
        cursor_style: CursorStyle::Block,
        theme,
        statusline,
    }
}

fn surface(theme: &Theme) -> Surface {
    Surface::new(20, 4, theme.base.foreground, theme.base.background)
}

#[test]
fn colors_error_and_warning_notices() {
    let theme = Theme::default();
    let viewport = Viewport::new(2, 20);
    let statusline = StatusBarConfig::default();
    for (severity, color) in [
        (MessageSeverity::Error, theme.diagnostic_error),
        (MessageSeverity::Warning, theme.diagnostic_warning),
    ] {
        let frame = notice_frame("E492: nope", severity, &viewport, &theme, &statusline);
        let mut out = surface(&theme);
        draw_message_row(&mut out, &frame).expect("draw");
        assert_eq!(out.cell(0, 3).expect("cell").style().foreground(), color);
    }
}

#[test]
fn renders_success_without_color() {
    let theme = Theme::default();
    let viewport = Viewport::new(2, 20);
    let statusline = StatusBarConfig::default();
    let frame = notice_frame(
        "saved",
        MessageSeverity::Success,
        &viewport,
        &theme,
        &statusline,
    );
    let mut out = surface(&theme);
    draw_message_row(&mut out, &frame).expect("draw");
    let style = out.cell(0, 3).expect("cell").style();
    assert_eq!(style.foreground(), theme.base.foreground);
    assert!(style.is_bold());
}

#[test]
fn leaves_command_text_untinted() {
    let theme = Theme::default();
    let viewport = Viewport::new(2, 20);
    let statusline = StatusBarConfig::default();
    let mut frame = notice_frame(
        "E492: nope",
        MessageSeverity::Error,
        &viewport,
        &theme,
        &statusline,
    );
    frame.command = Some("w");
    let mut out = surface(&theme);
    draw_message_row(&mut out, &frame).expect("draw");
    let style = out.cell(0, 3).expect("cell").style();
    assert_eq!(style.foreground(), theme.base.foreground);
    assert!(!style.is_bold());
}

#[test]
fn leaves_search_text_untinted() {
    let theme = Theme::default();
    let viewport = Viewport::new(2, 20);
    let statusline = StatusBarConfig::default();
    let mut frame = notice_frame(
        "E492: nope",
        MessageSeverity::Error,
        &viewport,
        &theme,
        &statusline,
    );
    frame.search = Some("alpha");
    let mut out = surface(&theme);
    draw_message_row(&mut out, &frame).expect("draw");
    assert_eq!(
        out.cell(0, 3).expect("cell").style().foreground(),
        theme.base.foreground
    );
}

#[test]
fn leaves_prompt_text_untinted() {
    let theme = Theme::default();
    let viewport = Viewport::new(2, 20);
    let statusline = StatusBarConfig::default();
    let mut frame = notice_frame(
        "E492: nope",
        MessageSeverity::Error,
        &viewport,
        &theme,
        &statusline,
    );
    frame.prompt = Some(PromptMenu {
        label: "rename",
        value: "value",
        cursor: 5,
    });
    let mut out = surface(&theme);
    draw_message_row(&mut out, &frame).expect("draw");
    assert_eq!(
        out.cell(0, 3).expect("cell").style().foreground(),
        theme.base.foreground
    );
}

#[test]
fn leaves_search_status_untinted() {
    let theme = Theme::default();
    let viewport = Viewport::new(2, 20);
    let statusline = StatusBarConfig::default();
    let mut frame = notice_frame(
        "E492: nope",
        MessageSeverity::Error,
        &viewport,
        &theme,
        &statusline,
    );
    frame.message = None;
    frame.search_status = Some("/ar [1/3]");
    let mut out = surface(&theme);
    draw_message_row(&mut out, &frame).expect("draw");
    assert_eq!(
        out.cell(0, 3).expect("cell").style().foreground(),
        theme.base.foreground
    );
}

#[test]
fn layout_prefers_prompt_over_command() {
    let prompt = PromptMenu {
        label: "rename",
        value: "value",
        cursor: 5,
    };
    let layout = layout_message_row_with_prompt(
        Some(&prompt),
        MessageParts {
            command: Some("w"),
            search: None,
            delimiter: '/',
            message: Some("saved"),
            search_status: None,
        },
        None,
        20,
    );
    assert_eq!(layout.line, "rename: value");
}

#[test]
fn layout_prefers_command_over_message() {
    let layout = layout_message_row(Some("w"), None, '/', Some("saved"), None, None, 20);
    assert_eq!(layout.line, ":w");
    assert_eq!(layout.showcmd_col, None);
}

#[test]
fn layout_right_aligns_showcmd() {
    let layout = layout_message_row(None, None, '/', Some("saved"), None, Some("d"), 20);
    assert_eq!(layout.line, "saved");
    assert_eq!(layout.showcmd, "d");
    assert_eq!(layout.showcmd_col, Some(19));
}

#[test]
fn layout_prefixes_backward_search_prompt() {
    let layout = layout_message_row(None, Some("foo"), '?', None, None, None, 20);
    assert_eq!(layout.line, "?foo");
}

#[test]
fn layout_truncates_by_display_width() {
    let layout = layout_message_row(
        None,
        None,
        '/',
        Some("\u{4e2d}\u{4e2d}\u{4e2d}"),
        None,
        None,
        4,
    );
    assert_eq!(layout.line, "\u{4e2d}\u{4e2d}");
}

#[test]
fn layout_truncates_left_for_showcmd() {
    let layout = layout_message_row(
        None,
        None,
        '/',
        Some("12345678901234567890"),
        None,
        Some("dd"),
        20,
    );
    assert_eq!(layout.line.chars().count(), 18);
    assert_eq!(layout.showcmd_col, Some(18));
}

#[test]
fn layout_empty_row_has_no_column() {
    let layout = layout_message_row(None, None, '/', None, None, None, 20);
    assert_eq!(layout.line, "");
    assert_eq!(layout.showcmd_col, None);
}

#[test]
fn layout_prefers_search_over_message() {
    let layout = layout_message_row(None, Some("ar"), '/', Some("saved"), None, None, 20);
    assert_eq!(layout.line, "/ar");
}

#[test]
fn layout_shows_search_status_when_idle() {
    let layout = layout_message_row(None, None, '/', None, Some("/ar [1/3]"), None, 20);
    assert_eq!(layout.line, "/ar [1/3]");
}

#[test]
fn layout_prefers_message_over_search_status() {
    let layout = layout_message_row(None, None, '/', Some("saved"), Some("/ar [1/3]"), None, 20);
    assert_eq!(layout.line, "saved");
}
